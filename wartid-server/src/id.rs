use std::fmt::Formatter;
use std::io::Write;
use std::marker::PhantomData;
use std::str::FromStr;

use diesel::backend::Backend;
use diesel::expression::bound::Bound;
use diesel::expression::AsExpression;
use diesel::pg::Pg;
use diesel::types::{FromSql, ToSql};
use diesel::{sql_types, Expression};
use rocket::form::error::ErrorKind;
use rocket::form::{FromFormField, ValueField};
use rocket::request::FromParam;
use uuid::Uuid;

pub struct Id<T>(Uuid, PhantomData<fn() -> T>);

impl<T> Id<T> {
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid, PhantomData)
    }

    #[inline]
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<T>();
        let type_name_short = type_name
            .rsplit_once("::")
            .map(|(_, n)| n)
            .unwrap_or(&type_name)
            .trim();

        f.debug_tuple(&format!("Id<{}>", type_name_short))
            .field(&self.0)
            .finish()
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl<T> FromStr for Id<T> {
    type Err = uuid::Error;

    fn from_str(uuid: &str) -> Result<Self, Self::Err> {
        uuid.parse().map(Self::from_uuid)
    }
}

impl<T> serde::Serialize for Id<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Uuid::serialize(&self.0, serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Uuid::deserialize(deserializer).map(Self::from_uuid)
    }
}

impl<'a, T> FromParam<'a> for Id<T> {
    type Error = ::uuid::Error;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.parse()
    }
}

#[rocket::async_trait]
impl<'r, T> FromFormField<'r> for Id<T> {
    fn from_value(field: ValueField<'r>) -> rocket::form::Result<'r, Self> {
        field
            .value
            .parse()
            .map_err(|err| ErrorKind::Custom(Box::new(err)).into())
    }
}

impl<T> ToSql<sql_types::Uuid, Pg> for Id<T> {
    fn to_sql<W: Write>(
        &self,
        out: &mut diesel::serialize::Output<W, Pg>,
    ) -> diesel::serialize::Result {
        <Uuid as diesel::serialize::ToSql<sql_types::Uuid, Pg>>::to_sql(&self.0, out)
    }
}

impl<T, ST> AsExpression<ST> for Id<T>
where
    Bound<ST, Uuid>: Expression<SqlType = ST>,
{
    type Expression = Bound<ST, Uuid>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self.0)
    }
}

impl<T, ST> AsExpression<ST> for &Id<T>
where
    Bound<ST, Uuid>: Expression<SqlType = ST>,
{
    type Expression = Bound<ST, Uuid>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self.0)
    }
}

impl<T> FromSql<sql_types::Uuid, Pg> for Id<T> {
    fn from_sql(bytes: Option<&<Pg as Backend>::RawValue>) -> diesel::deserialize::Result<Self> {
        Uuid::from_sql(bytes).map(Self::from_uuid)
    }
}

impl<T, ST, DB> diesel::query_source::Queryable<ST, DB> for Id<T>
where
    Uuid: diesel::types::FromSqlRow<ST, DB>,
    DB: diesel::backend::Backend,
    DB: diesel::types::HasSqlType<ST>,
{
    type Row = Uuid;

    fn build(row: Self::Row) -> Self {
        Self::from_uuid(row)
    }
}
