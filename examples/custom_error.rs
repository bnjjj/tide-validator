// #![feature(trait_alias)]
use async_std::io;
use async_std::task;
use serde::{Deserialize, Serialize};
use tide_validator::{ParameterType, ValidatorMiddleware};

#[derive(Deserialize, Serialize)]
struct Cat {
    name: String,
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let mut app = tide::new();

        let mut validator_middleware = ValidatorMiddleware::new();
        validator_middleware.add_validator(ParameterType::Param("n"), is_number);
        validator_middleware.add_validator(ParameterType::Header("X-Custom-Header"), is_number);
        validator_middleware.add_validator(ParameterType::QueryParam("test"), is_bool);
        validator_middleware.add_validator(ParameterType::Cookie("session"), is_required);
        validator_middleware.add_validator(ParameterType::Cookie("session"), is_length_under(20));

        // To access and let it works you have to launch it on localhost:8080/test/4 for example and put a cookie session
        app.at("/test/:n").middleware(validator_middleware).get(
            |_: tide::Request<()>| async move {
                let cat = Cat {
                    name: "Mozart".into(),
                };
                tide::Response::new(200).body_json(&cat).unwrap()
            },
        );

        app.listen("127.0.0.1:8080").await?;
        Ok(())
    })
}

#[derive(Debug, Serialize)]
struct CustomError {
    status_code: usize,
    message: String,
}

#[inline]
fn is_number(field_name: &str, field_value: Option<&str>) -> Result<(), CustomError> {
    if let Some(field_value) = field_value {
        if field_value.parse::<i64>().is_err() {
            return Err(CustomError {
                status_code: 400,
                message: format!(
                    "field '{}' = '{}' is not a valid number",
                    field_name, field_value
                ),
            });
        }
    }

    Ok(())
}

#[inline]
fn is_bool(field_name: &str, field_value: Option<&str>) -> Result<(), CustomError> {
    if let Some(field_value) = field_value {
        match field_value {
            "true" | "false" => return Ok(()),
            other => {
                return Err(CustomError {
                    status_code: 400,
                    message: format!(
                        "field '{}' = '{}' is not a valid boolean",
                        field_name, other
                    ),
                })
            }
        }
    }
    Ok(())
}

#[inline]
fn is_required(field_name: &str, field_value: Option<&str>) -> Result<(), CustomError> {
    if field_value.is_none() {
        Err(CustomError {
            status_code: 400,
            message: format!("'{}' is mandatory", field_name),
        })
    } else {
        Ok(())
    }
}

fn is_length_under(
    max_length: usize,
) -> Box<dyn Fn(&str, Option<&str>) -> Result<(), CustomError> + Send + Sync + 'static> {
    Box::new(
        move |field_name: &str, field_value: Option<&str>| -> Result<(), CustomError> {
            if let Some(field_value) = field_value {
                if field_value.len() > max_length {
                    let my_error = CustomError {
                        status_code: 400,
                        message: format!(
                            "element '{} which is equals to '{}' have not the maximum length of {}",
                            field_name, field_value, max_length
                        ),
                    };
                    return Err(my_error);
                }
            }
            Ok(())
        },
    )
}
