use askama::Template;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, get_service, post},
    Form, Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::{thread, time};
use tower_http::services::ServeDir;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

static mut CONTACTS: Contacts = vec![];

#[tokio::main]
async fn main() {
    unsafe {
        CONTACTS = new_contacts();
    }

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();
    let app = Router::new()
        .route("/", get(root))
        .route("/contacts", post(add_contact))
        .route("/contacts/:id", delete(delete_contact))
        .nest_service("/images", get_service(ServeDir::new("images")))
        .nest_service("/css", get_service(ServeDir::new("css")))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct Contact {
    id: u32,
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct CreateContactForm {
    name: String,
    email: String,
}

static mut ID: u32 = 0;
fn new_contact(name: String, email: String) -> Contact {
    unsafe { ID += 1 };
    Contact {
        id: unsafe { ID },
        name,
        email,
    }
}

type Contacts = Vec<Contact>;

fn new_contacts() -> Contacts {
    vec![
        new_contact(String::from("John"), String::from("jd@gmail.com")),
        new_contact(String::from("Clara"), String::from("cd@gmail.com")),
    ]
}

fn has_email(contacts: &Contacts, email: &str) -> bool {
    contacts.iter().any(|contact| contact.email == email)
}

fn get_contact_idx(contacts: &Contacts, id: u32) -> Option<usize> {
    contacts.iter().position(|contact| contact.id == id)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    contacts: &'a Contacts,
    form_data: &'a FormData,
}

#[derive(Template)]
#[template(path = "oob-contact.html")]
struct OOBContactTemplate<'a> {
    contact: &'a Contact,
}

struct FormData {
    values: HashMap<String, String>,
    errors: HashMap<String, String>,
}

#[derive(Template)]
#[template(path = "form.html")]
struct FormTemplate<'a> {
    form_data: &'a FormData,
}

fn new_form_data() -> FormData {
    FormData {
        values: HashMap::new(),
        errors: HashMap::new(),
    }
}

async fn root() -> Html<String> {
    let template = IndexTemplate {
        contacts: unsafe { &CONTACTS },
        form_data: &new_form_data(),
    };
    let rendered = template.render().unwrap();
    return Html(rendered);
}

async fn add_contact(Form(contact_form): Form<CreateContactForm>) -> impl IntoResponse {
    let name = contact_form.name;
    let email = contact_form.email;

    if has_email(unsafe { &CONTACTS }, &email) {
        let mut form_data = new_form_data();
        form_data.values.insert(String::from("name"), name);
        form_data.values.insert(String::from("email"), email);
        form_data
            .errors
            .insert(String::from("email"), String::from("Email already exists"));
        let template = FormTemplate {
            form_data: &form_data,
        };
        let rendered = template.render().unwrap();
        return (StatusCode::UNPROCESSABLE_ENTITY, Html(rendered));
    }

    let contact = new_contact(name, email);
    unsafe {
        CONTACTS.push(contact.clone());
    }

    let form_template = FormTemplate {
        form_data: &new_form_data(),
    };
    let rendered_form = form_template.render().unwrap();
    let oob_template = OOBContactTemplate { contact: &contact };
    let rendered_obb = oob_template.render().unwrap();
    return (
        StatusCode::OK,
        Html(format!("{rendered_form}\n{rendered_obb}")),
    );
}

async fn delete_contact(Path(contact_id): Path<u32>) -> impl IntoResponse {
    // Imitation of slow request to show an indicator
    thread::sleep(time::Duration::from_millis(3000));

    let contact_idx = get_contact_idx(unsafe { &CONTACTS }, contact_id);
    match contact_idx {
        None => return StatusCode::NOT_FOUND,
        Some(idx) => {
            unsafe { CONTACTS.remove(idx) };
            return StatusCode::OK;
        }
    }
}
