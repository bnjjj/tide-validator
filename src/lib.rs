//! tide-validator is a middleware working with [Tide](https://github.com/http-rs/tide), a web framework in Rust
//! which let you validate your data coming from a request. You'll be able
//! to create custom validators to validate your HTTP parameters, query parameters,
//! cookies and headers.
//!
//! # Features
//!
//! - __Custom validators:__ you can chain multiple validators and develop a custom validator is very easy. It's just a closure.
//! - __Validate everything:__ with the enum `HttpField` you can validate different fields like cookies, headers, query parameters and parameters.
//! - __Your own errors:__ thanks to generics in Rust you can use your own custom error when the data is invalid.
//!     need.
//!
//! # Validators
//!
//! To create your own validator it's just a closure to create with this form:
//!
//! ```rust,no_run,compile_fail
//! // The first closure's parameter is the parameter/queryparameter/cookie/header name.
//! // The second parameter is the value of this HTTP element. None means the field doesn't exist in the request (useful to force specific fields to be required).
//! Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static where T: Serialize + Send + Sync + 'static
//! ```
//!
//! # Examples
//!
//! __simple validation__
//! ```rust,no_run,compile_fail
//! // Our own validator is a simple closure to check if the field is a number
//! fn is_number(field_name: &str, field_value: Option<&str>) -> Result<(), String> {
//!     if let Some(field_value) = field_value {
//!         if field_value.parse::<i64>().is_err() {
//!             return Err(format!("field '{}' = '{}' is not a valid number", field_name, field_value));
//!         }
//!     }
//!
//!     Ok(())
//! }
//!
//! //... in main function
//! let mut app = tide::new();
//! let mut validator_middleware = ValidatorMiddleware::new();
//! // 'age' is the parameter name inside the route '/test/:age'
//! validator_middleware.add_validator(HttpField::Param("age"), is_number);
//! // You can assign different middleware for each routes therefore different validators for each routes
//! app.at("/test/:age")
//!     .middleware(validator_middleware)
//!     .get(|_: tide::Request<()>| async move {
//!         let cat = Cat {
//!             name: "Gribouille".into(),
//!         };
//!         Ok(tide::Response::new(StatusCode::Ok).body_json(&cat).unwrap())
//!      });
//! app.listen("127.0.0.1:8080").await?;
//! ```
//!
//! __chain multiple validators__
//! ```rust,no_run,compile_fail
//! // This validator force element to be required
//! fn is_required(field_name: &str, field_value: Option<&str>) -> Result<(), String> {
//!     if field_value.is_none() {
//!         Err(format!("'{}' is required", field_name))
//!     } else {
//!         Ok(())
//!     }
//! }
//!
//! // ... your main function
//!
//! let mut app = tide::new();
//! let mut validator_middleware = ValidatorMiddleware::new();
//! // Here 'age' is a query parameter, the validator stay the same as in previous example
//! validator_middleware.add_validator(HttpField::QueryParam("age"), is_number);
//! // You can also add multiple validators on a single query parameter to check different things
//! validator_middleware.add_validator(HttpField::QueryParam("age"), is_required);
//!
//! // You can assign different middleware for each routes therefore different validators for each routes
//! app.at("/test")
//!     .middleware(validator_middleware)
//!     .get(|_: tide::Request<()>| async move {
//!            let cat = Cat {
//!                 name: "Mozart".into(),
//!            };
//!            Ok(tide::Response::new(StatusCode::Ok).body_json(&cat).unwrap())
//!         },
//!     );
//!
//! app.listen("127.0.0.1:8080").await?;
//! ```
//!
//! __Use your own custom error__
//! ```rust,no_run,compile_fail
//! // Your custom error which your api will send if an error occurs
//! #[derive(Debug, Serialize)]
//! struct CustomError {
//!     status_code: usize,
//!     message: String,
//! }
//!
//! // Your validator can also return your own error type
//! fn is_number(field_name: &str, field_value: Option<&str>) -> Result<(), CustomError> {
//!     if let Some(field_value) = field_value {
//!         if field_value.parse::<i64>().is_err() {
//!             return Err(CustomError {
//!                 status_code: 400,
//!                 message: format!(
//!                     "field '{}' = '{}' is not a valid number",
//!                     field_name, field_value
//!                 ),
//!             });
//!         }
//!     }
//!     Ok(())
//! }
//!
//! // ... your main function
//! ```
//!
//! __Dynamic validators__
//! ```rust,no_run,compile_fail
//! // Validator inside a function as a closure to be dynamic with max_length
//! fn is_length_under(
//!     max_length: usize,
//! ) -> Box<dyn Fn(&str, Option<&str>) -> Result<(), CustomError> + Send + Sync + 'static> {
//!     Box::new(
//!         move |field_name: &str, field_value: Option<&str>| -> Result<(), CustomError> {
//!             if let Some(field_value) = field_value {
//!                 if field_value.len() > max_length {
//!                     let my_error = CustomError {
//!                         status_code: 400,
//!                         message: format!(
//!                             "element '{} which is equals to '{}' have not the maximum length of {}",
//!                             field_name, field_value, max_length
//!                         ),
//!                     };
//!                     return Err(my_error);
//!                 }
//!             }
//!             Ok(())
//!         },
//!     )
//! }
//!
//! // Simply call it on a cookie `session` for example:
//! validator_middleware.add_validator(HttpField::Cookie("session"), is_length_under(20));
//!
//! ```
//!
//! For more details about examples check out [the `examples` directory on GitHub](https://github.com/bnjjj/tide-validator/tree/master/examples)

use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::{fmt::Debug, sync::Arc};
use tide::{http::headers::HeaderName, Body, Middleware, Next, Request, Response, StatusCode};

// trait Validator = Fn(&str) -> Result<(), String> + Send + Sync + 'static;

/// Enum to indicate on which HTTP field you want to make validations
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum HttpField<'a> {
    /// To validate a path parameter. Example in URL `/test/:name` you can use `HttpField::Param("name")`
    Param(&'a str),
    /// To validate a query parameter. Example in URL `/test?name=test` you can use `HttpField::QueryParam("name")`
    QueryParam(&'a str),
    /// To validate a header. Example `HttpField::Header("X-My-Custom-Header")`
    Header(&'a str),
    /// To validate a cookie. Example `HttpField::Cookie("session")`
    Cookie(&'a str),
}

/// Used as a middleware in your tide framework and add your custom validators
pub struct ValidatorMiddleware<T>
where
    T: Serialize + Send + Sync + 'static,
{
    validators: HashMap<
        HttpField<'static>,
        Vec<Arc<dyn Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static>>,
    >,
}
impl<T> Debug for ValidatorMiddleware<T>
where
    T: Serialize + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("validators keys {:?}", self.validators.keys()))
    }
}

impl<T> ValidatorMiddleware<T>
where
    T: Serialize + Send + Sync + 'static,
{
    pub fn new() -> Self {
        ValidatorMiddleware {
            validators: HashMap::new(),
        }
    }

    pub fn with_validators<F>(mut self, validators: HashMap<HttpField<'static>, F>) -> Self
    where
        F: Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static,
    {
        for (param_name, validator) in validators {
            self.add_validator(param_name, validator);
        }
        self
    }
    pub fn add_validator<F>(&mut self, param_name: HttpField<'static>, validator: F)
    where
        F: Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static,
    {
        let validator = Arc::new(validator);
        let validator_moved = Arc::clone(&validator);
        self.validators
            .entry(param_name.into())
            .and_modify(|e| e.push(validator_moved))
            .or_insert(vec![validator]);
    }
}

#[tide::utils::async_trait]
impl<State, T> Middleware<State> for ValidatorMiddleware<T>
where
    State: Clone + Send + Sync + 'static,
    T: Serialize + Send + Sync + 'static,
{
    async fn handle(&self, ctx: Request<State>, next: Next<'_, State>) -> tide::Result {
        let mut query_parameters: Option<HashMap<String, String>> = None;
        for (param_name, validators) in &self.validators {
            match param_name {
                HttpField::Param(param_name) => {
                    for validator in validators {
                        // let param_found = ctx.param(param_name).unwrap();
                        // // let opt: Option<String> = Some(param_found.to_owned());
                        // // let value = opt.as_ref().map(|x| &**x).unwrap_or("");
                        // if let Err(err) = validator(param_name, Some(param_found)) {
                        //     let mut response = Response::new(StatusCode::BadRequest);
                        //     let body_json = Body::from_json(&json!(&err))?;
                        //     response.set_body(body_json);
                        //     return Ok(response);
                        // }

                        match ctx.param(param_name) {
                            Err(_err) => {
                        
                                return Ok(Response::new(StatusCode::BadRequest));
                            }
                            Ok(param_found) => {
                                if let Err(err) = validator(param_name, Some(param_found)) {
                                    let mut response = Response::new(StatusCode::BadRequest);
                                    let body_json = Body::from_json(&json!(&err))?;
                                    response.set_body(body_json);
                                    return Ok(response);
                                }
                            }
                        }

                        // if let Err(err) = validator(param_name, Some(param_found)) {
                        //     let mut response = Response::new(StatusCode::NoContent);
                        //     let body_json = Body::from_json(&json!(&err))?;
                        //     response.set_body(body_json);
                        //     return Ok(response);
                        // }
                    }
                }
                HttpField::QueryParam(param_name) => {
                    if query_parameters.is_none() {
                        match ctx.query::<HashMap<String, String>>() {
                            Err(_err) => return Ok(Response::new(StatusCode::InternalServerError)),
                            Ok(qps) => query_parameters = Some(qps),
                        }
                    }
                    let query_parameters = query_parameters.as_ref().unwrap();

                    for validator in validators {
                        if let Err(_err) = validator(
                            param_name,
                            query_parameters.get(&param_name[..]).map(|p| &p[..]),
                        ) {
                            return Ok(Response::new(StatusCode::InternalServerError));
                        }
                    }
                }
                HttpField::Header(header_name) => {
                    for validator in validators {
                        let x_header = &HeaderName::from_str(header_name).unwrap();
                        let header_found = ctx.header(x_header.as_str());
                        let c = header_found.map(|h| h.last().as_str());

                        if let Err(_err) = validator(header_name, c) {
                            return Ok(Response::new(StatusCode::BadRequest));
                        }
                    }
                }
                HttpField::Cookie(cookie_name) => {
                    for validator in validators {
                        let cookie_found = ctx.cookie(cookie_name);
                        if let Err(_err) =
                            validator(cookie_name, cookie_found.as_ref().map(|c| c.value()))
                        {
                            return Ok(Response::new(StatusCode::BadRequest));
                        }
                    }
                }
            }
        }
        Ok(next.run(ctx).await)
    }
}
