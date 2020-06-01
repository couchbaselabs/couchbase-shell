use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use async_stream::stream;
use fake::faker::address::raw::*;
use fake::faker::boolean::raw::*;
use fake::faker::chrono::raw::*;
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
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use serde_json::{from_value, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tera::{Context, Tera};
use uuid::Uuid;
use async_trait::async_trait;

pub struct FakeData {
    state: Arc<State>,
}

impl FakeData {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for FakeData {
    fn name(&self) -> &str {
        "fake"
    }

    fn signature(&self) -> Signature {
        Signature::build("fake")
            .named("template", SyntaxShape::Path, "path to the template", None)
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
    }

    fn usage(&self) -> &str {
        "Creates fake data from a template"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_fake(self.state.clone(), args, registry).await
    }
}

async fn run_fake(
    _state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let list_functions = args.get("list-functions").is_some();

    let ctx = Context::new();
    let mut tera = Tera::default();

    register_functions(&mut tera);

    if list_functions {
        let generated = tera.render_str(LIST_FUNCTIONS, &ctx).unwrap();
        let content = serde_json::from_str(&generated).unwrap();
        match content {
            serde_json::Value::Array(values) => {
                let stream = stream! {
                    for value in values {
                        let content_converted = convert_json_value_to_nu_value(&value, Tag::default());
                        yield content_converted;
                        //results.push(content_converted);
                    }
                };

                return Ok(OutputStream::from_input(stream));
            }
            _ => unimplemented!(),
        }
    } else {
        let path = args.get("template").ok_or_else(|| {
            ShellError::labeled_error(
                "No file or directory specified",
                "for command",
                Tag::default(),
            )
        })?;

        let num_rows = args
            .get("num-rows")
            .map(|v| v.as_u64().unwrap())
            .unwrap_or(1);
        let path = path.as_path().unwrap();
        let template = fs::read_to_string(path).unwrap();

        let stream = stream! {
            for _ in 0..num_rows {
                let generated = tera.render_str(&template, &ctx).unwrap();
                let content = serde_json::from_str(&generated).unwrap();
                let content_converted = convert_json_value_to_nu_value(&content, Tag::default());
                yield content_converted
            }
        };
        Ok(OutputStream::from_input(stream))
    }
}

fn register_functions(tera: &mut Tera) {
    // Group "misc"
    tera.register_function("uuid", |_: &HashMap<String, Value>| {
        Ok(Value::from(format!("{}", Uuid::new_v4())))
    });

    // Group "name"
    tera.register_function("name", |_: &HashMap<String, Value>| {
        Ok(Value::from(Name(EN).fake::<String>()))
    });
    tera.register_function("firstName", |_: &HashMap<String, Value>| {
        Ok(Value::from(FirstName(EN).fake::<String>()))
    });
    tera.register_function("lastName", |_: &HashMap<String, Value>| {
        Ok(Value::from(LastName(EN).fake::<String>()))
    });
    tera.register_function("title", |_: &HashMap<String, Value>| {
        Ok(Value::from(Title(EN).fake::<String>()))
    });
    tera.register_function("nameWithTitle", |_: &HashMap<String, Value>| {
        Ok(Value::from(NameWithTitle(EN).fake::<String>()))
    });
    tera.register_function("suffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(Suffix(EN).fake::<String>()))
    });

    // Group "internet"
    tera.register_function("color", |_: &HashMap<String, Value>| {
        Ok(Value::from(Color(EN).fake::<String>()))
    });
    tera.register_function("domainSuffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(DomainSuffix(EN).fake::<String>()))
    });
    tera.register_function("freeEmail", |_: &HashMap<String, Value>| {
        Ok(Value::from(FreeEmail(EN).fake::<String>()))
    });
    tera.register_function("freeEmailProvider", |_: &HashMap<String, Value>| {
        Ok(Value::from(FreeEmailProvider(EN).fake::<String>()))
    });
    tera.register_function("ipV4", |_: &HashMap<String, Value>| {
        Ok(Value::from(IPv4(EN).fake::<String>()))
    });
    tera.register_function("ipV6", |_: &HashMap<String, Value>| {
        Ok(Value::from(IPv6(EN).fake::<String>()))
    });
    tera.register_function("password", |args: &HashMap<String, Value>| {
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
        Ok(Value::from(
            Password(EN, length..length + 1).fake::<String>(),
        ))
    });
    tera.register_function("safeEmail", |_: &HashMap<String, Value>| {
        Ok(Value::from(SafeEmail(EN).fake::<String>()))
    });
    tera.register_function("userAgent", |_: &HashMap<String, Value>| {
        Ok(Value::from(UserAgent(EN).fake::<String>()))
    });
    tera.register_function("userName", |_: &HashMap<String, Value>| {
        Ok(Value::from(Username(EN).fake::<String>()))
    });

    // Group "number"
    tera.register_function("digit", |_: &HashMap<String, Value>| {
        Ok(Value::from(Digit(EN).fake::<String>()))
    });
    tera.register_function("numberWithFormat", |args: &HashMap<String, Value>| {
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
        Ok(Value::from(NumberWithFormat(EN, format_str).fake::<String>()))
    });

    // Group "boolean"
    tera.register_function("bool", |args: &HashMap<String, Value>| {
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
        Ok(Value::from(Boolean(EN, ratio).fake::<bool>()))
    });

    // Group "company"
    tera.register_function("companySuffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(CompanySuffix(EN).fake::<String>()))
    });
    tera.register_function("companyName", |_: &HashMap<String, Value>| {
        Ok(Value::from(CompanyName(EN).fake::<String>()))
    });
    tera.register_function("buzzword", |_: &HashMap<String, Value>| {
        Ok(Value::from(Buzzword(EN).fake::<String>()))
    });
    tera.register_function("catchphrase", |_: &HashMap<String, Value>| {
        Ok(Value::from(CatchPhase(EN).fake::<String>()))
    });
    tera.register_function("bs", |_: &HashMap<String, Value>| {
        Ok(Value::from(Bs(EN).fake::<String>()))
    });
    tera.register_function("profession", |_: &HashMap<String, Value>| {
        Ok(Value::from(Profession(EN).fake::<String>()))
    });
    tera.register_function("industry", |_: &HashMap<String, Value>| {
        Ok(Value::from(Industry(EN).fake::<String>()))
    });

    // Group "address"
    tera.register_function("cityPrefix", |_: &HashMap<String, Value>| {
        Ok(Value::from(CityPrefix(EN).fake::<String>()))
    });
    tera.register_function("citySuffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(CitySuffix(EN).fake::<String>()))
    });
    tera.register_function("cityName", |_: &HashMap<String, Value>| {
        Ok(Value::from(CityName(EN).fake::<String>()))
    });
    tera.register_function("countryName", |_: &HashMap<String, Value>| {
        Ok(Value::from(CountryName(EN).fake::<String>()))
    });
    tera.register_function("countryCode", |_: &HashMap<String, Value>| {
        Ok(Value::from(CountryCode(EN).fake::<String>()))
    });
    tera.register_function("streetSuffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(StreetSuffix(EN).fake::<String>()))
    });
    tera.register_function("streetName", |_: &HashMap<String, Value>| {
        Ok(Value::from(StreetName(EN).fake::<String>()))
    });
    tera.register_function("timeZone", |_: &HashMap<String, Value>| {
        Ok(Value::from(TimeZone(EN).fake::<String>()))
    });
    tera.register_function("stateName", |_: &HashMap<String, Value>| {
        Ok(Value::from(StateName(EN).fake::<String>()))
    });
    tera.register_function("stateAbbr", |_: &HashMap<String, Value>| {
        Ok(Value::from(StateAbbr(EN).fake::<String>()))
    });
    tera.register_function("zipCode", |_: &HashMap<String, Value>| {
        Ok(Value::from(ZipCode(EN).fake::<String>()))
    });
    tera.register_function("postCode", |_: &HashMap<String, Value>| {
        Ok(Value::from(PostCode(EN).fake::<String>()))
    });
    tera.register_function("buildingNumber", |_: &HashMap<String, Value>| {
        Ok(Value::from(BuildingNumber(EN).fake::<String>()))
    });
    tera.register_function("latitude", |_: &HashMap<String, Value>| {
        Ok(Value::from(Latitude(EN).fake::<String>()))
    });
    tera.register_function("longitude", |_: &HashMap<String, Value>| {
        Ok(Value::from(Longitude(EN).fake::<String>()))
    });

    // Group "phone_number"
    tera.register_function("phoneNumber", |_: &HashMap<String, Value>| {
        Ok(Value::from(PhoneNumber(EN).fake::<String>()))
    });
    tera.register_function("cellNumber", |_: &HashMap<String, Value>| {
        Ok(Value::from(CellNumber(EN).fake::<String>()))
    });

    // Group "datetime"
    tera.register_function("time", |_: &HashMap<String, Value>| {
        Ok(Value::from(Time(EN).fake::<String>()))
    });
    tera.register_function("date", |_: &HashMap<String, Value>| {
        Ok(Value::from(Date(EN).fake::<String>()))
    });
    tera.register_function("dateTime", |_: &HashMap<String, Value>| {
        Ok(Value::from(DateTime(EN).fake::<String>()))
    });
    tera.register_function("duration", |_: &HashMap<String, Value>| {
        Ok(Value::from(
            Duration(EN).fake::<chrono::Duration>().num_milliseconds(),
        ))
    });

    // Group "filesystem"
    tera.register_function("filePath", |_: &HashMap<String, Value>| {
        Ok(Value::from(FilePath(EN).fake::<String>()))
    });
    tera.register_function("fileName", |_: &HashMap<String, Value>| {
        Ok(Value::from(FileName(EN).fake::<String>()))
    });
    tera.register_function("fileExtension", |_: &HashMap<String, Value>| {
        Ok(Value::from(FileExtension(EN).fake::<String>()))
    });
    tera.register_function("dirPath", |_: &HashMap<String, Value>| {
        Ok(Value::from(DirPath(EN).fake::<String>()))
    });

    // Group "currency"
    tera.register_function("currencyCode", |_: &HashMap<String, Value>| {
        Ok(Value::from(CurrencyCode(EN).fake::<String>()))
    });
    tera.register_function("currencyName", |_: &HashMap<String, Value>| {
        Ok(Value::from(CurrencyName(EN).fake::<String>()))
    });
    tera.register_function("currencySymbol", |_: &HashMap<String, Value>| {
        Ok(Value::from(CurrencySymbol(EN).fake::<String>()))
    });

    // Group "lorem"
    tera.register_function("words", |args: &HashMap<String, Value>| {
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
        Ok(Value::from(words.join(" ")))
    });
    tera.register_function("sentences", |args: &HashMap<String, Value>| {
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
        Ok(Value::from(sentences.join(" ")))
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
    { "group": "filesystem", "name": "fileName())", "description": "File name", "example": "{{ fileName() }}" },
    { "group": "filesystem", "name": "fileExtension())", "description": "File extension", "example": "{{ fileExtension() }}" },
    { "group": "filesystem", "name": "dirPath())", "description": "Dir path", "example": "{{ dirPath() }}" },
    { "group": "currency", "name": "currencyCode())", "description": "Currency code", "example": "{{ currencyCode() }}" },
    { "group": "currency", "name": "currencyName())", "description": "Currency name", "example": "{{ currencyName() }}" },
    { "group": "currency", "name": "currencySymbol())", "description": "Currency symbol", "example": "{{ currencySymbol() }}" },
    { "group": "lorem", "name": "words(num=1))", "description": "Words", "example": "{{ words() }}" },
    { "group": "lorem", "name": "sentences(num=1))", "description": "Sentences", "example": "{{ sentences() }}" }
]"#;
