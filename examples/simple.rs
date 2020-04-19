// #![feature(trait_alias)]
use async_std::io;
use async_std::task;
use serde::{Deserialize, Serialize};
use tide::http_types::StatusCode;
use tide_validator::{HttpField, ValidatorMiddleware};

#[derive(Deserialize, Serialize)]
struct Cat {
    name: String,
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let mut app = tide::new();

        let mut validator_middleware = ValidatorMiddleware::new();
        // 'age' is the parameter name inside the route '/test/:name'
        validator_middleware.add_validator(HttpField::Param("age"), is_number);

        // You can also add multiple validators on a single parameter to check different things
        validator_middleware.add_validator(HttpField::Param("age"), is_required);

        // You can assign different middleware for each routes so different validators for each routes
        app.at("/test/:age").middleware(validator_middleware).get(
            |_: tide::Request<()>| async move {
                let cat = Cat {
                    name: "Gribouille".into(),
                };
                tide::Response::new(StatusCode::Ok).body_json(&cat).unwrap()
            },
        );

        app.listen("127.0.0.1:8080").await?;
        Ok(())
    })
}

#[inline]
fn is_number(field_name: &str, field_value: Option<&str>) -> Result<(), String> {
    if let Some(field_value) = field_value {
        if field_value.parse::<i64>().is_err() {
            return Err(format!(
                "field '{}' = '{}' is not a valid number",
                field_name, field_value
            ));
        }
    }

    Ok(())
}

#[inline]
fn is_required(field_name: &str, field_value: Option<&str>) -> Result<(), String> {
    if field_value.is_none() {
        Err(format!("'{}' is required", field_name))
    } else {
        Ok(())
    }
}
