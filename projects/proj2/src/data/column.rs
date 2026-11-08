#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Int64,
    Varchar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarcharColumn {
    pub offsets: Vec<u32>,
    pub data: Vec<u8>,
}

impl Default for VarcharColumn {
    fn default() -> Self {
        Self::new()
    }
}

impl VarcharColumn {
    pub fn new() -> Self {
        Self {
            offsets: vec![0],
            data: Vec::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(&self, i: usize) -> &[u8] {
        let start = self.offsets[i] as usize;
        let end = self.offsets[i + 1] as usize;
        &self.data[start..end]
    }

    #[inline]
    pub fn push(&mut self, s: &[u8]) {
        self.data.extend_from_slice(s);
        self.offsets.push(self.data.len() as u32);
    }

    pub fn clear(&mut self) {
        self.offsets.clear();
        self.offsets.push(0);
        self.data.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.offsets
            .windows(2)
            .map(move |w| &self.data[w[0] as usize..w[1] as usize])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Column {
    Int64(Vec<i64>),
    Varchar(VarcharColumn),
}

impl Column {
    pub fn new(ty: ColumnType) -> Self {
        match ty {
            ColumnType::Int64 => Column::Int64(Vec::new()),
            ColumnType::Varchar => Column::Varchar(VarcharColumn::new()),
        }
    }

    pub fn column_type(&self) -> ColumnType {
        match self {
            Column::Int64(_) => ColumnType::Int64,
            Column::Varchar(_) => ColumnType::Varchar,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Column::Int64(v) => v.len(),
            Column::Varchar(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        match self {
            Column::Int64(v) => v.clear(),
            Column::Varchar(v) => v.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varchar_push_get_roundtrip() {
        let mut col = VarcharColumn::new();
        assert_eq!(col.len(), 0);

        col.push(b"hello");
        col.push(b"");
        col.push(b"abc");

        assert_eq!(col.len(), 3);
        assert_eq!(col.offsets, vec![0, 5, 5, 8]);
        assert_eq!(col.data, b"helloabc");
        assert_eq!(col.get(0), b"hello");
        assert_eq!(col.get(1), b"");
        assert_eq!(col.get(2), b"abc");

        let all: Vec<&[u8]> = col.iter().collect();
        assert_eq!(all, vec![&b"hello"[..], &b""[..], &b"abc"[..]]);
    }

    #[test]
    fn varchar_clear_keeps_capacity() {
        let mut col = VarcharColumn::new();
        for _ in 0..100 {
            col.push(b"some string");
        }
        let cap_data = col.data.capacity();
        let cap_off = col.offsets.capacity();

        col.clear();
        assert_eq!(col.len(), 0);
        assert_eq!(col.offsets, vec![0]);
        assert_eq!(col.data.capacity(), cap_data);
        assert_eq!(col.offsets.capacity(), cap_off);
    }

    #[test]
    fn column_enum_basics() {
        let mut c = Column::new(ColumnType::Int64);
        assert_eq!(c.column_type(), ColumnType::Int64);
        assert!(c.is_empty());
        if let Column::Int64(v) = &mut c {
            v.extend([1, -2, i64::MAX]);
        }
        assert_eq!(c.len(), 3);
        c.clear();
        assert!(c.is_empty());

        let mut s = Column::new(ColumnType::Varchar);
        assert_eq!(s.column_type(), ColumnType::Varchar);
        if let Column::Varchar(v) = &mut s {
            v.push(b"x");
        }
        assert_eq!(s.len(), 1);
    }
}
