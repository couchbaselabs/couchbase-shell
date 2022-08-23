use super::util::convert_json_value_to_nu_value;
use crate::cli::error::{generic_error, serialize_error};
use crate::state::State;
use fake::faker::address::raw::*;
use fake::faker::boolean::raw::*;
use fake::faker::chrono::raw::*;
use fake::faker::color::raw::HexColor;
use fake::faker::company::raw::*;
use fake::faker::currency::raw::*;
use fake::faker::filesystem::raw::*;
use fake::faker::internet::raw::*;
use fake::faker::lorem::raw::*;
use fake::faker::name::raw::*;
use fake::faker::number::raw::*;
use fake::faker::phone_number::raw::*;
use fake::locales::*;
use fake::Fake;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use serde_json::from_value;
use serde_json::Value as JSONValue;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};
use uuid::Uuid;

#[derive(Clone)]
pub struct FakeData {
    state: Arc<Mutex<State>>,
}

impl FakeData {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for FakeData {
    fn name(&self) -> &str {
        "fake"
    }

    fn signature(&self) -> Signature {
        Signature::build("fake")
            .named(
                "template",
                SyntaxShape::String,
                "path to the template",
                None,
            )
            .named(
                "num-rows",
                SyntaxShape::Int,
                "number of rows to generate",
                None,
            )
            .switch(
                "list-functions",
                "List all functions currently registered",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Creates fake data from a template"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_fake(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run_fake(
    _state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let list_functions = call.has_flag("list-functions");

    let ctx = Context::new();
    let mut tera = Tera::default();

    register_functions(&mut tera);

    if list_functions {
        let generated = tera
            .render_str(LIST_FUNCTIONS, &ctx)
            .map_err(|e| generic_error(format!("Failed to render functions {}", e), None, span))?;
        let content = serde_json::from_str(&generated)
            .map_err(|e| generic_error(format!("Failed to render functions {}", e), None, span))?;
        match content {
            serde_json::Value::Array(values) => {
                let converted: Vec<Value> = values
                    .into_iter()
                    .map(|v| match convert_json_value_to_nu_value(&v, span) {
                        Ok(c) => Ok(c),
                        Err(e) => Err(e),
                    })
                    .collect::<Result<Vec<Value>, ShellError>>()?;

                Ok(Value::List {
                    vals: converted,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            _ => unimplemented!(),
        }
    } else {
        let path: String = match call.get_flag(engine_state, stack, "template")? {
            Some(p) => p,
            None => return Err(ShellError::MissingParameter("template".to_string(), span)),
        };

        let num_rows: i64 = call.get_flag(engine_state, stack, "num-rows")?.unwrap_or(1);

        let template = fs::read_to_string(path).map_err(|e| {
            generic_error(
                format!("Failed to read template file {}", e),
                "Is the path to the file correct?".to_string(),
                span,
            )
        })?;

        let converted = std::iter::repeat_with(move || {
            let rendered = tera.render_str(&template, &ctx).map_err(|e| {
                generic_error(format!("Failed to render template {}", e), None, span)
            })?;
            let generated = serde_json::from_str(&rendered)
                .map_err(|e| serialize_error(e.to_string(), span))?;
            let converted = convert_json_value_to_nu_value(&generated, span)?;

            Ok(converted)
            // return match tera.render_str(&template, &ctx) {
            //     Ok(generated) => match serde_json::from_str(&generated) {
            //         Ok(content) => match convert_json_value_to_nu_value(&content, span) {
            //             Ok(c) => Ok(ReturnSuccess::Value(c)),
            //             Err(e) => Err(e),
            //         },
            //         Err(e) => Err(ShellError::unexpected(format!("{}", e))),
            //     },
            //     Err(e) => Err(ShellError::unexpected(format!("{}", e))),
            // };
        })
        .take(num_rows as usize)
        .collect::<Result<Vec<Value>, ShellError>>()?;

        Ok(Value::List {
            vals: converted,
            span: call.head,
        }
        .into_pipeline_data())
    }
}

fn register_functions(tera: &mut Tera) {
    // Group "misc"
    tera.register_function("uuid", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(format!("{}", Uuid::new_v4())))
    });

    // Group "name"
    tera.register_function("name", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Name(EN).fake::<String>()))
    });
    tera.register_function("firstName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(FirstName(EN).fake::<String>()))
    });
    tera.register_function("lastName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(LastName(EN).fake::<String>()))
    });
    tera.register_function("title", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Title(EN).fake::<String>()))
    });
    tera.register_function("nameWithTitle", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(NameWithTitle(EN).fake::<String>()))
    });
    tera.register_function("suffix", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Suffix(EN).fake::<String>()))
    });

    // Group "internet"
    tera.register_function("color", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(HexColor(EN).fake::<String>()))
    });
    tera.register_function("domainSuffix", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(DomainSuffix(EN).fake::<String>()))
    });
    tera.register_function("freeEmail", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(FreeEmail(EN).fake::<String>()))
    });
    tera.register_function("freeEmailProvider", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(FreeEmailProvider(EN).fake::<String>()))
    });
    tera.register_function("ipV4", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(IPv4(EN).fake::<String>()))
    });
    tera.register_function("ipV6", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(IPv6(EN).fake::<String>()))
    });
    tera.register_function("password", |args: &HashMap<String, JSONValue>| {
        let length = match args.get("length") {
            Some(val) => match from_value::<usize>(val.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(tera::Error::msg(format!(
                        "Function `password` received length={} but `length` can only be a number",
                        val
                    )));
                }
            },
            None => 10,
        };
        Ok(JSONValue::from(
            Password(EN, length..length + 1).fake::<String>(),
        ))
    });
    tera.register_function("safeEmail", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(SafeEmail(EN).fake::<String>()))
    });
    tera.register_function("userAgent", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(UserAgent(EN).fake::<String>()))
    });
    tera.register_function("userName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Username(EN).fake::<String>()))
    });

    // Group "number"
    tera.register_function("digit", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Digit(EN).fake::<String>()))
    });
    tera.register_function("numberWithFormat", |args: &HashMap<String, JSONValue>| {
        let format = match args.get("format") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(tera::Error::msg(format!(
                        "Function `numberWithFormat` received format={} but `format` can only be a string",
                        val
                    )));
                }
            },
            None => String::from("#"),
        };
        // We need to convert String to &'static str here so we need to use a leak.
        // This is a known leak which should only be called once and is small.
        let format_str = Box::leak(format.into_boxed_str());
        Ok(JSONValue::from(NumberWithFormat(EN, format_str).fake::<String>()))
    });

    // Group "boolean"
    tera.register_function("bool", |args: &HashMap<String, JSONValue>| {
        let ratio = match args.get("ratio") {
            Some(val) => match from_value::<u8>(val.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(tera::Error::msg(format!(
                        "Function `ratio` received ratio={} but `ratio` can only be a u8",
                        val
                    )));
                }
            },
            None => 50,
        };
        Ok(JSONValue::from(Boolean(EN, ratio).fake::<bool>()))
    });

    // Group "company"
    tera.register_function("companySuffix", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CompanySuffix(EN).fake::<String>()))
    });
    tera.register_function("companyName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CompanyName(EN).fake::<String>()))
    });
    tera.register_function("buzzword", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Buzzword(EN).fake::<String>()))
    });
    tera.register_function("catchphrase", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CatchPhase(EN).fake::<String>()))
    });
    tera.register_function("bs", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Bs(EN).fake::<String>()))
    });
    tera.register_function("profession", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Profession(EN).fake::<String>()))
    });
    tera.register_function("industry", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Industry(EN).fake::<String>()))
    });

    // Group "address"
    tera.register_function("cityPrefix", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CityPrefix(EN).fake::<String>()))
    });
    tera.register_function("citySuffix", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CitySuffix(EN).fake::<String>()))
    });
    tera.register_function("cityName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CityName(EN).fake::<String>()))
    });
    tera.register_function("countryName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CountryName(EN).fake::<String>()))
    });
    tera.register_function("countryCode", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CountryCode(EN).fake::<String>()))
    });
    tera.register_function("streetSuffix", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(StreetSuffix(EN).fake::<String>()))
    });
    tera.register_function("streetName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(StreetName(EN).fake::<String>()))
    });
    tera.register_function("timeZone", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(TimeZone(EN).fake::<String>()))
    });
    tera.register_function("stateName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(StateName(EN).fake::<String>()))
    });
    tera.register_function("stateAbbr", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(StateAbbr(EN).fake::<String>()))
    });
    tera.register_function("zipCode", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(ZipCode(EN).fake::<String>()))
    });
    tera.register_function("postCode", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(PostCode(EN).fake::<String>()))
    });
    tera.register_function("buildingNumber", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(BuildingNumber(EN).fake::<String>()))
    });
    tera.register_function("latitude", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Latitude(EN).fake::<String>()))
    });
    tera.register_function("longitude", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Longitude(EN).fake::<String>()))
    });

    // Group "phone_number"
    tera.register_function("phoneNumber", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(PhoneNumber(EN).fake::<String>()))
    });
    tera.register_function("cellNumber", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CellNumber(EN).fake::<String>()))
    });

    // Group "datetime"
    tera.register_function("time", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Time(EN).fake::<String>()))
    });
    tera.register_function("date", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(Date(EN).fake::<String>()))
    });
    tera.register_function("dateTime", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(DateTime(EN).fake::<String>()))
    });
    tera.register_function("duration", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(
            Duration(EN).fake::<chrono::Duration>().num_milliseconds(),
        ))
    });

    // Group "filesystem"
    tera.register_function("filePath", |_: &HashMap<String, JSONValue>| {
        // We need to escape this string because it contains a path, in Windows paths
        // are backslashes, which unescaped cause serde_json to error when parsing
        // the generated string in --list-functions.
        Ok(JSONValue::from(
            FilePath(EN).fake::<String>().escape_default().to_string(),
        ))
    });
    tera.register_function("fileName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(FileName(EN).fake::<String>()))
    });
    tera.register_function("fileExtension", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(FileExtension(EN).fake::<String>()))
    });
    tera.register_function("dirPath", |_: &HashMap<String, JSONValue>| {
        // We need to escape this string because it contains a path, in Windows paths
        // are backslashes, which unescaped cause serde_json to error when parsing
        // the generated string in --list-functions.
        Ok(JSONValue::from(
            DirPath(EN).fake::<String>().escape_default().to_string(),
        ))
    });

    // Group "currency"
    tera.register_function("currencyCode", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CurrencyCode(EN).fake::<String>()))
    });
    tera.register_function("currencyName", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CurrencyName(EN).fake::<String>()))
    });
    tera.register_function("currencySymbol", |_: &HashMap<String, JSONValue>| {
        Ok(JSONValue::from(CurrencySymbol(EN).fake::<String>()))
    });

    // Group "lorem"
    tera.register_function("words", |args: &HashMap<String, JSONValue>| {
        let num = match args.get("num") {
            Some(val) => match from_value::<usize>(val.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(tera::Error::msg(format!(
                        "Function `words` received num={} but `num` can only be a number",
                        val
                    )));
                }
            },
            None => 1,
        };
        let words = Words(EN, num..num + 1).fake::<Vec<String>>();
        Ok(JSONValue::from(words.join(" ")))
    });
    tera.register_function("sentences", |args: &HashMap<String, JSONValue>| {
        let num = match args.get("num") {
            Some(val) => match from_value::<usize>(val.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(tera::Error::msg(format!(
                        "Function `sentences` received num={} but `num` can only be a number",
                        val
                    )));
                }
            },
            None => 1,
        };
        let sentences = Sentences(EN, num..num + 1).fake::<Vec<String>>();
        Ok(JSONValue::from(sentences.join(" ")))
    });
}

static LIST_FUNCTIONS: &str = r#"[
    { "group": "misc", "name": "uuid()", "description": "UUID v4", "example": "{{ uuid() }}" },
    { "group": "name", "name": "firstName()", "description": "First name", "example": "{{ firstName() }}" },
    { "group": "name", "name": "lastName()", "description": "Last name", "example": "{{ lastName() }}" },
    { "group": "name", "name": "name()", "description": "firstName and lastName combined", "example": "{{ name() }}" },
    { "group": "name", "name": "title()", "description": "Person title", "example": "{{ title() }}" },
    { "group": "name", "name": "nameWithTitle()", "description": "name and title combined", "example": "{{ nameWithTitle() }}" },
    { "group": "name", "name": "suffix()", "description": "Person info/degree", "example": "{{ suffix() }}" },
    { "group": "internet", "name": "color()", "description": "Color hex code", "example": "{{ color() }}" },
    { "group": "internet", "name": "domainSuffix()", "description": "Domain suffix", "example": "{{ domainSuffix() }}" },
    { "group": "internet", "name": "freeEmail()", "description": "Email that might exist", "example": "{{ freeEmail() }}" },
    { "group": "internet", "name": "freeEmailProvider()", "description": "Provider that exists", "example": "{{ freeEmailProvider() }}" },
    { "group": "internet", "name": "ipV4()", "description": "IP v4 address", "example": "{{ ipV4() }}" },
    { "group": "internet", "name": "ipV6()", "description": "IP v6 address", "example": "{{ ipV6() }}" },
    { "group": "internet", "name": "userAgent()", "description": "User Agent", "example": "{{ userAgent() }}" },
    { "group": "internet", "name": "safeEmail()", "description": "Email that does not exist", "example": "{{ safeEmail() }}" },
    { "group": "internet", "name": "userName()", "description": "Username", "example": "{{ userName() }}" },
    { "group": "internet", "name": "password(length=10)", "description": "Password", "example": "{{ password() }}" },
    { "group": "number", "name": "digit()", "description": "Digit", "example": "{{ digit() }}" },
    { "group": "number", "name": "numberWithFormat(format='#')", "description": "Number with format (escape with '')", "example": "{{ numberWithFormat(format='^###') }}" },
    { "group": "boolean", "name": "bool(ratio=50)", "description": "Boolean", "example": "{{ bool() }}" },
    { "group": "company", "name": "companyName()", "description": "Company name", "example": "{{ companyName() }}" },
    { "group": "company", "name": "companySuffix()", "description": "Company suffix", "example": "{{ companySuffix() }}" },
    { "group": "company", "name": "buzzword()", "description": "Business related buzzword", "example": "{{ buzzword() }}" },
    { "group": "company", "name": "catchphrase()", "description": "Business related catchphrase", "example": "{{ catchphrase() }}" },
    { "group": "company", "name": "bs()", "description": "Business related bs", "example": "{{ bs() }}" },
    { "group": "company", "name": "profession()", "description": "Profession", "example": "{{ profession() }}" },
    { "group": "company", "name": "industry()", "description": "Industry", "example": "{{ industry() }}" },
    { "group": "address", "name": "cityPrefix()", "description": "City prefix", "example": "{{ cityPrefix() }}" },
    { "group": "address", "name": "citySuffix()", "description": "City suffix", "example": "{{ citySuffix() }}" },
    { "group": "address", "name": "cityName()", "description": "City name", "example": "{{ cityName() }}" },
    { "group": "address", "name": "countryName()", "description": "Country name", "example": "{{ countryName() }}" },
    { "group": "address", "name": "countryCode()", "description": "Country code", "example": "{{ countryCode() }}" },
    { "group": "address", "name": "streetSuffix()", "description": "Street suffix", "example": "{{ streetSuffix() }}" },
    { "group": "address", "name": "streetName()", "description": "Street name", "example": "{{ streetName() }}" },
    { "group": "address", "name": "timeZone()", "description": "Timezone", "example": "{{ timeZone() }}" },
    { "group": "address", "name": "stateName()", "description": "State name", "example": "{{ stateName() }}" },
    { "group": "address", "name": "stateAbbr()", "description": "State abbreviation", "example": "{{ stateAbbr() }}" },
    { "group": "address", "name": "zipCode()", "description": "Zip code", "example": "{{ zipCode() }}" },
    { "group": "address", "name": "postCode()", "description": "Post code", "example": "{{ postCode() }}" },
    { "group": "address", "name": "buildingNumber()", "description": "Building number", "example": "{{ buildingNumber() }}" },
    { "group": "address", "name": "latitude()", "description": "Latitude", "example": "{{ latitude() }}" },
    { "group": "address", "name": "longitude()", "description": "Longitude", "example": "{{ longitude() }}" },
    { "group": "phone_number", "name": "phoneNumber()", "description": "Phone number", "example": "{{ phoneNumber() }}" },
    { "group": "phone_number", "name": "cellNumber()", "description": "Cell number", "example": "{{ cellNumber() }}" },
    { "group": "datetime", "name": "time()", "description": "Time", "example": "{{ time() }}" },
    { "group": "datetime", "name": "date()", "description": "Date", "example": "{{ date() }}" },
    { "group": "datetime", "name": "dateTime()", "description": "DateTime", "example": "{{ dateTime() }}" },
    { "group": "datetime", "name": "duration()", "description": "Duration (ms)", "example": "{{ duration() }}" },
    { "group": "filesystem", "name": "filePath()", "description": "File path", "example": "{{ filePath() }}" },
    { "group": "filesystem", "name": "fileName()", "description": "File name", "example": "{{ fileName() }}" },
    { "group": "filesystem", "name": "fileExtension()", "description": "File extension", "example": "{{ fileExtension() }}" },
    { "group": "filesystem", "name": "dirPath()", "description": "Dir path", "example": "{{ dirPath() }}" },
    { "group": "currency", "name": "currencyCode()", "description": "Currency code", "example": "{{ currencyCode() }}" },
    { "group": "currency", "name": "currencyName()", "description": "Currency name", "example": "{{ currencyName() }}" },
    { "group": "currency", "name": "currencySymbol()", "description": "Currency symbol", "example": "{{ currencySymbol() }}" },
    { "group": "lorem", "name": "words(num=1)", "description": "Words", "example": "{{ words() }}" },
    { "group": "lorem", "name": "sentences(num=1)", "description": "Sentences", "example": "{{ sentences() }}" }
]"#;
