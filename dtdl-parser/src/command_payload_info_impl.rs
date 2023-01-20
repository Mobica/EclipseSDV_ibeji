// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT license.

use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;

use crate::command_payload_info::CommandPayloadInfo;
use crate::dtmi::Dtmi;
use crate::entity_info::EntityInfo;
use crate::entity_kind::EntityKind;
use crate::named_entity_info::NamedEntityInfo;
use crate::schema_field_info::SchemaFieldInfo;
use crate::schema_info::SchemaInfo;

pub struct CommandPayloadInfoImpl {
    // EntityInfo
    dtdl_version: i32,
    id: Dtmi,
    child_of: Option<Dtmi>,
    defined_in: Option<Dtmi>,
    undefined_properties: HashMap<String, Value>,

    // NamedEntityInfo
    name: Option<String>,    

    // SchemaFieldInfo
    schema: Option<Box<dyn SchemaInfo>>,
}

impl CommandPayloadInfoImpl {
    /// Returns a new CommandPayloadInfoImpl.
    ///
    /// # Arguments
    /// * `dtdl_version` - The DTDL version used to define the command payload.
    /// * `id` - The identifier.
    /// * `child_of` - The identifier of the parent element in which this command payload is defined.
    /// * `defined_in` - The identifier of the partition in which this command payload is defined.
    /// * `name` - The name.
    /// * `schema` - The schema.
    pub fn new(
        dtdl_version: i32,
        id: Dtmi,
        child_of: Option<Dtmi>,
        defined_in: Option<Dtmi>,
        name: Option<String>,
        schema: Option<Box<dyn SchemaInfo>>
    ) -> Self {
        Self {
            dtdl_version,
            id,
            child_of,
            defined_in,
            undefined_properties: HashMap::<String, Value>::new(),
            name,            
            schema,
        }
    }
}

impl EntityInfo for CommandPayloadInfoImpl {
    /// Returns the DTDL version.
    fn dtdl_version(&self) -> i32 {
        self.dtdl_version
    }

    /// Returns the identifier.
    fn id(&self) -> &Dtmi {
        &self.id
    }

    /// Returns the kind of entity.
    fn entity_kind(&self) -> EntityKind {
        EntityKind::CommandPayload
    }

    /// Returns the parent's identifier.
    fn child_of(&self) -> &Option<Dtmi> {
        &self.child_of
    }

    /// Returns the enclosing partition's identifider.
    fn defined_in(&self) -> &Option<Dtmi> {
        &self.defined_in
    }

    /// Returns all undefined properties.
    fn undefined_properties(&self) -> &HashMap<String, Value> {
        &self.undefined_properties
    }

    /// Add an undefined property.
    /// # Arguments
    /// * `key` - The property's name.
    /// * `value` - The property's value.
    fn add_undefined_property(&mut self, key: String, value: Value) {
        self.undefined_properties.insert(key, value);
    }

    /// Returns the instance as an Any.
    fn as_any(&self) -> &dyn Any {
        self
    }     
}

impl NamedEntityInfo for CommandPayloadInfoImpl {  
    /// Returns the name.
    fn name(&self) -> &Option<String> {
        &self.name
    }  
}

impl SchemaFieldInfo for CommandPayloadInfoImpl {
    /// Returns the schema.
    fn schema(&self) -> &Option<Box<dyn SchemaInfo>> {
        &self.schema
    }    
}

impl CommandPayloadInfo for CommandPayloadInfoImpl {  
}

#[cfg(test)]
mod command_payload_info_impl_tests {
    use super::*;
    use crate::dtmi::{create_dtmi, Dtmi};
    use crate::model_parser::DTDL_VERSION;
    use crate::primitive_schema_info_impl::PrimitiveSchemaInfoImpl;
    use serde_json;

    #[test]
    fn new_command_payload_info_impl_test() {
        let mut id_result: Option<Dtmi> = None;
        create_dtmi("dtmi:com:example:send_notification:request;1.0", &mut id_result);
        assert!(id_result.is_some());
        let id = id_result.unwrap();

        let mut child_of_result: Option<Dtmi> = None;
        create_dtmi("dtmi:com:example:HVAC;1.0", &mut child_of_result);
        assert!(child_of_result.is_some());
        let child_of = child_of_result.unwrap();

        let mut defined_in_result: Option<Dtmi> = None;
        create_dtmi("dtmi:com:example:command.send_notification;1.0", &mut defined_in_result);
        assert!(defined_in_result.is_some());
        let defined_in = defined_in_result.unwrap();

        let first_propery_value: Value = serde_json::from_str("{\"first\": \"this\"}").unwrap();
        let second_propery_value: Value = serde_json::from_str("{\"second\": \"that\"}").unwrap();

        let mut schema_info_id: Option<Dtmi> = None;
        create_dtmi("dtmi:dtdl:class:String;2", &mut schema_info_id);
        assert!(schema_info_id.is_some());

        let boxed_schema_info = Box::new(PrimitiveSchemaInfoImpl::new(DTDL_VERSION, schema_info_id.unwrap(), None, None, EntityKind::String));        

        let mut command_payload_info = CommandPayloadInfoImpl::new(
            2,
            id.clone(),
            Some(child_of.clone()),
            Some(defined_in.clone()),
            Some(String::from("one")),            
            Some(boxed_schema_info),
        );
        command_payload_info.add_undefined_property(String::from("first"), first_propery_value.clone());
        command_payload_info.add_undefined_property(String::from("second"), second_propery_value.clone());

        assert!(command_payload_info.dtdl_version() == 2);
        assert!(command_payload_info.id() == &id);
        assert!(command_payload_info.child_of().is_some());
        assert!(command_payload_info.child_of().clone().unwrap() == child_of);
        assert!(command_payload_info.defined_in().is_some());
        assert!(command_payload_info.defined_in().clone().unwrap() == defined_in);
        assert!(command_payload_info.entity_kind() == EntityKind::CommandPayload);
        assert!(command_payload_info.undefined_properties().len() == 2);
        assert!(
            command_payload_info.undefined_properties().get("first").unwrap().clone() == first_propery_value
        );
        assert!(
            command_payload_info.undefined_properties().get("second").unwrap().clone()
                == second_propery_value
        );

        let retrieved_name = command_payload_info.name().clone(); 
        assert!(retrieved_name.is_some());
        assert!(retrieved_name.unwrap() == "one");    
    }
}
