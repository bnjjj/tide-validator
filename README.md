<h1 align="center">tide-validator</h1>
<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/tide-validator">
    <img src="https://img.shields.io/crates/v/tide-validator.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/tide-validator">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- github action status -->
  <a href="https://github.com/bnjjj/tide-validator/actions?query=workflow%3ARust">
    <img src="https://github.com/bnjjj/tide-validator/workflows/Rust/badge.svg"
      alt="github action status" />
  </a>
</div>

__tide-validator is a middleware working with [Tide](https://github.com/http-rs/tide), a web framework in Rust which let you validate your data coming from a request. You'll be able to create custom validators to validate your HTTP parameters, query parameters, cookies and headers.__

# Features

- __Custom validators:__ you can chain multiple validators and develop a custom validator is very easy. It's just a closure.
- __Validate everything:__ with the enum `HttpField` you can validate different fields like cookies, headers, query parameters and parameters.
- __Your own errors:__ thanks to generics in Rust you can use your own custom error when the data is invalid.
    need.

# Validators

To create your own validator it's just a closure to create with this form:

```rust
// The first closure's parameter is the parameter/queryparameter/cookie/header name.
// The second parameter is the value of this HTTP element.
// None means the field doesn't exist in the request (useful to force specific fields to be required).
Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static where T: Serialize + Send + Sync + 'static
```

# Documentation

The full documentation is available [here](https://docs.rs/tide-validator)

# Examples

+ __simple validation__
```rust
// Our own validator is a simple closure to check if the field is a number
fn is_number(field_name: &str, field_value: Option<&str>) -> Result<(), String> {
    if let Some(field_value) = field_value {
        if field_value.parse::<i64>().is_err() {
            return Err(format!("field '{}' = '{}' is not a valid number", field_name, field_value));
        }
    }

    Ok(())
}

//... in main function
let mut app = tide::new();
let mut validator_middleware = ValidatorMiddleware::new();
// 'age' is the parameter name inside the route '/test/:age'
validator_middleware.add_validator(HttpField::Param("age"), is_number);
// You can assign different middleware for each routes therefore different validators for each routes
app.at("/test/:age")
    .middleware(validator_middleware)
    .get(|_: tide::Request<()>| async move {
        let cat = Cat {
            name: "Gribouille".into(),
        };
        tide::Response::new(StatusCode::Ok).body_json(&cat).unwrap()
     });
app.listen("127.0.0.1:8080").await?;
```

+ __chain multiple validators__
```rust
// This validator force element to be required
fn is_required(field_name: &str, field_value: Option<&str>) -> Result<(), String> {
    if field_value.is_none() {
        Err(format!("'{}' is required", field_name))
    } else {
        Ok(())
    }
}

// ... your main function

let mut app = tide::new();
let mut validator_middleware = ValidatorMiddleware::new();
// Here 'age' is a query parameter, the validator stay the same as in previous example
validator_middleware.add_validator(HttpField::QueryParam("age"), is_number);
// You can also add multiple validators on a single query parameter to check different things
validator_middleware.add_validator(HttpField::QueryParam("age"), is_required);

// You can assign different middleware for each routes therefore different validators for each routes
app.at("/test")
    .middleware(validator_middleware)
    .get(|_: tide::Request<()>| async move {
           let cat = Cat {
                name: "Mozart".into(),
           };
            tide::Response::new(StatusCode::Ok).body_json(&cat).unwrap()
        },
    );

app.listen("127.0.0.1:8080").await?;
```

+ __Use your own custom error__
```rust
// Your custom error which your api will send if an error occurs
#[derive(Debug, Serialize)]
struct CustomError {
    status_code: usize,
    message: String,
}

// Your validator can also return your own error type
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

// ... your main function
```

+ __Dynamic validators__
```rust
// Validator inside a function as a closure to be dynamic with max_length
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

// Simply call it on a cookie `session` for example:
validator_middleware.add_validator(HttpField::Cookie("session"), is_length_under(20));
```

