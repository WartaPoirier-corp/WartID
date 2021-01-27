use uuid::Uuid;

#[macro_export]
macro_rules! def_id {
    (pub struct $name:ident) => {
        def_id!(@impl $name [pub struct $name(::uuid::Uuid);]);
    };
    (struct $name:ident) => {
        def_id!(@impl $name [struct $name(::uuid::Uuid);]);
    };
    (@impl $name:ident [$($struct:tt)*]) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, SqlType, FromSqlRow, AsExpression, serde::Deserialize, serde::Serialize)]
        #[sql_type = "diesel::sql_types::Uuid"]
        $($struct)*

        impl $name {
            #[inline]
            pub fn into_inner(self) -> ::uuid::Uuid {
                self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = ::uuid::Error;
            fn from_str(uuid: &str) -> Result<Self, Self::Err> {
                uuid.parse().map(Self)
            }
        }

        impl<'a> ::rocket::request::FromParam<'a> for $name {
            type Error = ::uuid::Error;

            fn from_param(param: &'a ::rocket::http::RawStr) -> Result<Self, Self::Error> {
                param.parse()
            }
        }

        impl<'a> ::rocket::request::FromFormValue<'a> for $name {
            type Error = ::uuid::Error;

            fn from_form_value(form_value: &'a ::rocket::http::RawStr) -> Result<Self, Self::Error> {
                form_value.parse()
            }
        }

        impl ::diesel::deserialize::FromSql<::diesel::sql_types::Uuid, ::diesel::pg::Pg> for $name {
            fn from_sql(bytes: Option<&<::diesel::pg::Pg as ::diesel::backend::Backend>::RawValue>) -> ::diesel::deserialize::Result<Self> {
                ::uuid::Uuid::from_sql(bytes).map(Self)
            }
        }

        impl ::diesel::serialize::ToSql<::diesel::sql_types::Uuid, ::diesel::pg::Pg> for $name {
            fn to_sql<W: ::std::io::Write>(&self, out: &mut ::diesel::serialize::Output<W, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                <::uuid::Uuid as ::diesel::serialize::ToSql<::diesel::sql_types::Uuid, ::diesel::pg::Pg>>
                    ::to_sql(&self.0, out)
            }
        }
    };
}
