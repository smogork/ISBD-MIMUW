use super::ColumnType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: ColumnType,
}

impl Field {
    pub fn new(name: impl Into<String>, ty: ColumnType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    fields: Vec<Field>,
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn field(&self, i: usize) -> &Field {
        &self.fields[i]
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    pub fn project(&self, indices: &[usize]) -> Schema {
        Schema::new(indices.iter().map(|&i| self.fields[i].clone()).collect())
    }

    pub fn column_types(&self) -> impl Iterator<Item = ColumnType> + '_ {
        self.fields.iter().map(|f| f.ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_lookup() {
        let s = Schema::new(vec![
            Field::new("id", ColumnType::Int64),
            Field::new("name", ColumnType::Varchar),
        ]);
        assert_eq!(s.len(), 2);
        assert_eq!(s.index_of("name"), Some(1));
        assert_eq!(s.index_of("nope"), None);
        assert_eq!(s.field(0).ty, ColumnType::Int64);
        let types: Vec<_> = s.column_types().collect();
        assert_eq!(types, vec![ColumnType::Int64, ColumnType::Varchar]);

        let p = s.project(&[1, 0, 1]);
        let names: Vec<_> = p.fields().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["name", "id", "name"]);
    }
}
