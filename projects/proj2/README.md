# ISBD columnar file format

## Build

```bash
make         
make test 
```

## Write (CSV -> file)

```bash
target/release/isbd-write input.csv out.isbd \
    --schema "id:INT64,name:VARCHAR,city:VARCHAR,score:INT64" --header
```

`--schema-file path` accepts a file with one `name TYPE` per line instead of
`--schema`. `--header` skips the first CSV row. Row group size is controlled by
`--max-rows` (default 65536) and `--max-bytes` (default 1 MiB).

## Read (file -> metrics)

```bash
target/release/isbd-read out.isbd
```

Sample file with two `INT64` and two `VARCHAR` columns: `examples/sample.isbd`
(5000 rows, built from `examples/sample.csv` by `make sample`).

## File layout

```
"ISBD" | version u16
row group 0: chunk of column 0 | chunk of column 1 | ...
row group 1: ...
footer | footer length u32 | "ISBD"
```

A row group holds up to N rows of every column; a chunk is one column of one
row group. The footer stores the schema and, for every chunk, its offset and
compressed/uncompressed size.

Chunk encodings:

* `INT64`: delta from the previous value, zigzag, LEB128 varint.
* `VARCHAR`: varint total byte length, one varint per string length, then one
  LZ4 block with all string bytes concatenated.

## In-memory representation

```rust
enum Column {
    Int64(Vec<i64>),
    Varchar(VarcharColumn),   // offsets: Vec<u32>, data: Vec<u8>
}
struct RecordBatch { columns: Vec<Column> }   // all columns have the same length
```

A `RecordBatch` corresponds to one row group. Strings are stored as one
contiguous byte buffer plus offsets; string `i` is `data[offsets[i]..offsets[i+1]]`.
The reader memory-maps the file and decodes chunks straight into the batch's
vectors, reusing their allocations across row groups.
