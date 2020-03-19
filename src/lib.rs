use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use serde::Serialize;
use tide::{Middleware, Next, Request, Response};
// trait Validator = Fn(&str) -> Result<(), String> + Send + Sync + 'static;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ParameterType<'a> {
    Param(&'a str),
    QueryParam(&'a str),
    Header(&'a str),
    Cookie(&'a str),
}

pub struct ValidatorMiddleware<T>
where
    T: Serialize + Send + Sync + 'static,
{
    validators: HashMap<
        ParameterType<'static>,
        Vec<Arc<dyn Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static>>,
    >,
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

    pub fn with_validators<F>(mut self, validators: HashMap<ParameterType<'static>, F>) -> Self
    where
        F: Fn(&str, Option<&str>) -> Result<(), T> + Send + Sync + 'static,
    {
        for (param_name, validator) in validators {
            self.add_validator(param_name, validator);
        }
        self
    }

    pub fn add_validator<F>(&mut self, param_name: ParameterType<'static>, validator: F)
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

impl<State, T> Middleware<State> for ValidatorMiddleware<T>
where
    State: Send + Sync + 'static,
    T: Serialize + Send + Sync + 'static,
{
    fn handle<'a>(&'a self, ctx: Request<State>, next: Next<'a, State>) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let mut query_parameters: Option<HashMap<String, String>> = None;

            for (param_name, validators) in &self.validators {
                match param_name {
                    ParameterType::Param(param_name) => {
                        for validator in validators {
                            let param_found: Result<String, _> = ctx.param(param_name);
                            if let Err(err) =
                                validator(param_name, param_found.ok().as_ref().map(|p| &p[..]))
                            {
                                return Response::new(400).body_json(&err).unwrap_or_else(
                                        |err| {
                                            return Response::new(500).body_string(format!(
                                                "cannot serialize your parameter validator for '{}' error : {:?}",
                                                param_name,
                                                err
                                            ));
                                        },
                                    );
                            }
                        }
                    }
                    ParameterType::QueryParam(param_name) => {
                        if query_parameters.is_none() {
                            match ctx.query::<HashMap<String, String>>() {
                                Err(err) => {
                                    return Response::new(500).body_string(format!(
                                        "cannot read query parameters: {:?}",
                                        err
                                    ))
                                }
                                Ok(qps) => query_parameters = Some(qps),
                            }
                        }
                        let query_parameters = query_parameters.as_ref().unwrap();

                        for validator in validators {
                            if let Err(err) = validator(
                                param_name,
                                query_parameters.get(&param_name[..]).map(|p| &p[..]),
                            ) {
                                return Response::new(400).body_json(&err).unwrap_or_else(
                                        |err| {
                                            return Response::new(500).body_string(format!(
                                                "cannot serialize your query parameter validator for '{}' error : {:?}",
                                                param_name,
                                                err
                                            ));
                                        },
                                    );
                            }
                        }
                    }
                    ParameterType::Header(header_name) => {
                        for validator in validators {
                            let header_found: Option<&str> = ctx.header(header_name);
                            if let Err(err) = validator(header_name, header_found) {
                                return Response::new(400).body_json(&err).unwrap_or_else(
                                        |err| {
                                            return Response::new(500).body_string(format!(
                                                "cannot serialize your header validator for '{}' error : {:?}",
                                                header_name,
                                                err
                                            ));
                                        },
                                    );
                            }
                        }
                    }
                    ParameterType::Cookie(cookie_name) => {
                        for validator in validators {
                            let cookie_found = ctx.cookie(cookie_name);
                            if let Err(err) =
                                validator(cookie_name, cookie_found.as_ref().map(|c| c.value()))
                            {
                                return Response::new(400).body_json(&err).unwrap_or_else(
                                        |err| {
                                            return Response::new(500).body_string(format!(
                                                "cannot serialize your cookie validator for '{}' error : {:?}",
                                                cookie_name,
                                                err
                                            ));
                                        },
                                    );
                            }
                        }
                    }
                }
            }
            next.run(ctx).await
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
