pub const NAME: [u8; 7] = *b"PARKIET";
pub const VERSION: u16 = 1;
pub const HEADER_LEN: usize = 9;

pub fn header() -> [u8; HEADER_LEN] {
    let [v0, v1] = VERSION.to_le_bytes();
    let [n0, n1, n2, n3, n4, n5, n6] = NAME;
    [n0, n1, n2, n3, n4, n5, n6, v0, v1]
}

#[derive(Debug)]
pub struct Footer<'a> {
    pub schema: Schema<'a>,
    // Rows of each row group
    pub group_rows: Vec<u64>,
    // `group_rows.len() * schema length` chunks
    pub chunks: Vec<Chunk>,
}

impl<'a> Footer<'a> {
    pub fn encode(self, destination: &mut Vec<u8>) {
        unimplemented!();
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug)]
pub struct Schema<'a> {
    fields: Vec<Field<'a>>,
}

impl<'a> Schema<'a> {
    pub fn new(fields: Vec<Field<'a>>) -> Self {
        // FIXME: validate
        Self { fields }
    }
}

#[derive(Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    pub data_type: DataType,
}

impl<'a> Field<'a> {
    pub const fn new(name: &'a str, data_type: DataType) -> Self {
        Self { name, data_type }
    }
}

#[derive(Debug)]
pub enum DataType {
    Int64,
    Varchar,
}

// Borrowed column: a slice of i64 and StrColumn
#[derive(Debug)]
pub enum Column<'a> {
    Int64(&'a [i64]),
    Varchar(StrColumn<'a>),
}

impl<'a> Column<'a> {
    pub fn len(&self) -> usize {
        match self {
            Self::Int64(values) => values.len(),
            Self::Varchar(values) => values.len(),
        }
    }
}

// Varchar column is one string where offsets indicate rows.
#[derive(Debug)]
pub struct StrColumn<'a> {
    data: &'a str,
    offsets: &'a [u32],
}

impl<'a> StrColumn<'a> {
    pub fn new(data: &'a str, offsets: &'a [u32]) -> Self {
        // FIXME: validate
        Self { data, offsets }
    }
    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }
}
