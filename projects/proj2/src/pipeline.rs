use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use crate::data::{RecordBatch, Schema};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub batches: u64,
    pub rows: u64,
}

pub fn run<E, P, C>(
    schema: &Schema,
    depth: usize,
    mut produce: P,
    mut consume: C,
) -> Result<Stats, E>
where
    E: Send,
    P: FnMut(&mut RecordBatch) -> Result<bool, E> + Send,
    C: FnMut(&RecordBatch) -> Result<(), E>,
{
    let depth = depth.max(1);
    let (full_tx, full_rx): (SyncSender<Result<RecordBatch, E>>, Receiver<_>) = sync_channel(depth);
    let (empty_tx, empty_rx): (SyncSender<RecordBatch>, Receiver<RecordBatch>) =
        sync_channel(depth + 1);
    for _ in 0..depth + 1 {
        empty_tx
            .send(RecordBatch::new(schema))
            .expect("fresh channel");
    }

    thread::scope(|s| {
        s.spawn(move || {
            while let Ok(mut batch) = empty_rx.recv() {
                match produce(&mut batch) {
                    Ok(true) => {
                        if full_tx.send(Ok(batch)).is_err() {
                            return;
                        }
                    }
                    Ok(false) => return,
                    Err(e) => {
                        let _ = full_tx.send(Err(e));
                        return;
                    }
                }
            }
        });

        let empty_tx = empty_tx;
        let mut stats = Stats::default();
        for item in full_rx {
            let batch = item?;
            stats.batches += 1;
            stats.rows += batch.num_rows() as u64;
            consume(&batch)?;
            let _ = empty_tx.send(batch);
        }
        Ok(stats)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ColumnType, Field};
    use std::time::{Duration, Instant};

    fn schema() -> Schema {
        Schema::new(vec![Field::new("x", ColumnType::Int64)])
    }

    fn counter(
        n: usize,
        rows: usize,
    ) -> impl FnMut(&mut RecordBatch) -> Result<bool, String> + Send {
        let mut next = 0i64;
        let mut produced = 0usize;
        move |b: &mut RecordBatch| {
            if produced == n {
                return Ok(false);
            }
            b.clear();
            let Column::Int64(v) = b.column_mut(0) else {
                unreachable!()
            };
            v.extend(next..next + rows as i64);
            next += rows as i64;
            produced += 1;
            Ok(true)
        }
    }

    #[test]
    fn delivers_all_batches_in_order() {
        let mut seen = Vec::new();
        let stats = run(&schema(), 2, counter(10, 100), |b| {
            let Column::Int64(v) = b.column(0) else {
                unreachable!()
            };
            seen.extend_from_slice(v);
            Ok::<(), String>(())
        })
        .unwrap();
        assert_eq!(
            stats,
            Stats {
                batches: 10,
                rows: 1000
            }
        );
        assert_eq!(seen, (0..1000).collect::<Vec<i64>>());
    }

    #[test]
    fn empty_producer() {
        let stats = run(&schema(), 2, counter(0, 100), |_| Ok::<(), String>(())).unwrap();
        assert_eq!(stats, Stats::default());
    }

    #[test]
    fn producer_error_propagates_after_earlier_batches() {
        let mut produced = 0;
        let produce = move |b: &mut RecordBatch| {
            produced += 1;
            if produced == 3 {
                return Err("boom".to_string());
            }
            b.clear();
            let Column::Int64(v) = b.column_mut(0) else {
                unreachable!()
            };
            v.push(produced);
            Ok(true)
        };
        let mut consumed = 0;
        let err = run(&schema(), 2, produce, |_| {
            consumed += 1;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(err, "boom");
        assert_eq!(consumed, 2);
    }

    #[test]
    fn consumer_error_stops_producer() {
        let mut consumed = 0;
        let err = run(&schema(), 2, counter(1_000_000, 10), |_| {
            consumed += 1;
            if consumed == 5 {
                Err("stop".to_string())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(err, "stop");
        assert_eq!(consumed, 5);
    }

    #[test]
    fn overlaps_producer_and_consumer() {
        let work = Duration::from_millis(20);
        let mut p = counter(10, 1);
        let produce = move |b: &mut RecordBatch| {
            thread::sleep(work);
            p(b)
        };
        let t = Instant::now();
        run(&schema(), 2, produce, |_| {
            thread::sleep(work);
            Ok::<(), String>(())
        })
        .unwrap();
        let elapsed = t.elapsed();
        assert!(elapsed < Duration::from_millis(330), "{elapsed:?}");
    }

    #[test]
    fn buffers_are_recycled() {
        let mut ptrs = std::collections::HashSet::new();
        run(&schema(), 2, counter(50, 1000), |b| {
            let Column::Int64(v) = b.column(0) else {
                unreachable!()
            };
            ptrs.insert(v.as_ptr() as usize);
            Ok::<(), String>(())
        })
        .unwrap();
        assert!(ptrs.len() <= 3, "{} distinct buffers", ptrs.len());
    }
}
