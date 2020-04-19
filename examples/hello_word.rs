// #![feature(trait_alias)]
use async_std::io;
use tide_validator::{HttpField, ValidatorMiddleware};
#[async_std::main]
async fn main() -> io::Result<()> {
        let mut app = tide::new();
        let mut validator_middleware = ValidatorMiddleware::new();
        let is_number = |_field_name: &str, field_value: Option<&str>| {
            field_value.unwrap().parse::<i64>().map(|_| ()).map_err(|_| "invalid number")
        };
        validator_middleware.add_validator(HttpField::Param("age"), is_number);
        app.at("/test/:age").middleware(validator_middleware).get(
            |_: tide::Request<()>| async move { "Hello World" },
        );

        app.listen("127.0.0.1:8080").await?;
        Ok(())
}
