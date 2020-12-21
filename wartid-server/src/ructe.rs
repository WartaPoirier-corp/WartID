use rocket::response::content::Html as HtmlContent;
use rocket::response::Responder;
use rocket::Request;

pub struct Ructe(pub Vec<u8>);

impl<'r> Responder<'r> for Ructe {
    fn respond_to(self, req: &Request) -> rocket::response::Result<'r> {
        HtmlContent(self.0).respond_to(req)
    }
}

#[macro_export]
macro_rules! render {
    ($group:tt::$page:tt($($param:expr),*)) => {
        {
            use crate::templates;

            let mut res = Vec::new();
            templates::$group::$page(
                &mut res,
                $(
                    $param
                ),*
            ).unwrap();

            Ructe(res)
        }
    }
}
