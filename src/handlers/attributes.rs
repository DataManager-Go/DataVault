use super::{authentication::Authenticateduser, requests::attribute::UpdateAttribute, utils};
use crate::{
    models::{
        attribute::{self, Attribute, AttributeType},
        namespace::Namespace,
    },
    response_code, DbConnection,
};
use crate::{
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};
use response_code::diesel_option;

struct RequestData {
    db: DbConnection,
    action: String,
    attr_type: AttributeType,
    namespace: Namespace,
    request: UpdateAttribute,
    user: Authenticateduser,
}

/// Endpoint for registering new users
pub async fn ep_list_attributes(
    pool: web::Data<DbPool>,
    attr_type: web::Path<String>,
    request: Json<UpdateAttribute>,
    user: Authenticateduser,
) -> Result<Json<Vec<String>>, RestError> {
    if request.namespace.is_empty() {
        return Err(RestError::BadRequest);
    }

    let namespace = utils::retrieve_namespace_by_name_async(
        pool.get()?,
        request.namespace.clone(),
        user.clone(),
    )
    .await?;

    let request_data = RequestData {
        db: pool.get()?,
        action: "get".to_string(),
        attr_type: to_attribute(attr_type.into_inner())?,
        namespace,
        request: request.clone(),
        user,
    };

    let res = web::block(move || request_data.get()).await??;

    Ok(Json(res))
}

/// Endpoint for registering new users
pub async fn ep_attribute_action(
    pool: web::Data<DbPool>,
    action: web::Path<(String, String)>,
    request: Json<UpdateAttribute>,
    user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    let (attr, action) = action.into_inner();

    validate_action(&action)?;
    validate_request(&action, &request)?;

    let namespace = utils::retrieve_namespace_by_name_async(
        pool.get()?,
        request.namespace.clone(),
        user.clone(),
    )
    .await?;

    let request_data = RequestData {
        db: pool.get()?,
        action,
        attr_type: to_attribute(attr)?,
        namespace,
        request: request.clone(),
        user,
    };

    web::block(move || request_data.run_action()).await??;

    Ok(SUCCESS)
}

impl RequestData {
    // Run the attribute request
    fn run_action(self) -> Result<(), RestError> {
        match self.action.as_str() {
            "create" => self.create()?,
            "delete" => self.delete()?,
            "update" => self.update()?,
            _ => unreachable!(),
        };

        Ok(())
    }

    // Updates a an attribute
    fn update(self) -> Result<(), RestError> {
        let mut attribute = self.find_attribute()?;
        attribute.name = self.request.new_name;
        attribute.save(&self.db)?;
        Ok(())
    }

    // Create a new attribute
    fn create(self) -> Result<(), RestError> {
        let new_attr = attribute::NewAttribute {
            name: self.request.name,
            type_: self.attr_type,
            user_id: self.user.user.id,
            namespace_id: self.namespace.id,
        };
        new_attr.create(&self.db)?;
        Ok(())
    }

    // Delete an existing attribute
    fn delete(self) -> Result<(), RestError> {
        self.find_attribute()?.delete(&self.db)?;
        Ok(())
    }

    /// List attributes in the requested namespace
    /// and of the requested type
    fn get(self) -> Result<Vec<String>, RestError> {
        Ok(attribute::list_names(
            &self.db,
            self.attr_type,
            self.namespace.id,
        )?)
    }

    /// Find an attribute based on the request
    fn find_attribute(&self) -> Result<Attribute, RestError> {
        attribute::NewAttribute::find_by_name(
            &self.db,
            &self.request.name,
            self.attr_type,
            self.user.user.id,
            self.namespace.id,
        )
        .map_err(|i| diesel_option(i, self.attr_type))
    }
}

fn validate_request(action: &str, request: &UpdateAttribute) -> Result<(), RestError> {
    if (action == "update" && request.new_name.is_empty())
        || request.namespace.is_empty()
        || request.name.is_empty()
    {
        return Err(RestError::BadRequest);
    }

    Ok(())
}

fn validate_action(action: &str) -> Result<(), RestError> {
    if matches!(action, "update" | "delete" | "create") {
        Ok(())
    } else {
        Err(RestError::BadRequest)
    }
}

fn to_attribute(attr: String) -> Result<AttributeType, RestError> {
    match attr.to_lowercase().as_str() {
        "tag" => Ok(AttributeType::Tag),
        "group" => Ok(AttributeType::Group),
        _ => Err(RestError::BadRequest),
    }
}
