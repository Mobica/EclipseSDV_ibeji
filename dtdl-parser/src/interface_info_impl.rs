// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT license.

use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;

use crate::dtmi::Dtmi;
use crate::entity_info::EntityInfo;
use crate::entity_kind::EntityKind;
use crate::interface_info::InterfaceInfo;

#[derive(Clone)]
pub struct InterfaceInfoImpl {
    // EntitytInfo
    dtdl_version: i32,
    id: Dtmi,
    child_of: Option<Dtmi>,
    defined_in: Option<Dtmi>,
    undefined_properties: HashMap<String, Value>,
}

impl InterfaceInfoImpl {
    /// Returns a new InterfaceInfoImpl.
    ///
    /// # Arguments
    /// * `dtdl_version` - The DTDL version used to define the interface.
    /// * `id` - The identifier.
    /// * `child_of` - The identifier of the parent element in which this interface is defined.
    /// * `defined_in` - The identifier of the partition in which this interface is defined.
    pub fn new(
        dtdl_version: i32,
        id: Dtmi,
        child_of: Option<Dtmi>,
        defined_in: Option<Dtmi>
    ) -> Self {
        Self {
            dtdl_version,
            id,
            child_of,
            defined_in,
            undefined_properties: HashMap::<String, Value>::new(),            
        }
    }
}

impl EntityInfo for InterfaceInfoImpl {
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
        EntityKind::Interface
    }

    /// Returns the parent's identifier.
    fn child_of(&self) -> &Option<Dtmi> {
        &self.child_of
    }

    /// Returns the enclosing partition's identifier.
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

impl InterfaceInfo for InterfaceInfoImpl {   
}

#[cfg(test)]
mod interface_info_impl_tests {
    use super::*;
    use crate::dtmi::{create_dtmi, Dtmi};
    use serde_json;

    #[test]
    fn new_interface_info_impl_test() {
        let mut id_result: Option<Dtmi> = None;
        create_dtmi("dtmi:com:example:Thermostat;1.0", &mut id_result);
        assert!(id_result.is_some());
        let id = id_result.unwrap();

        let mut child_of_result: Option<Dtmi> = None;
        create_dtmi("dtmi:com:example:Cabin;1.0", &mut child_of_result);
        assert!(child_of_result.is_some());
        let child_of = child_of_result.unwrap();

        let mut defined_in_result: Option<Dtmi> = None;
        create_dtmi("dtmi:com:example:Something;1.0", &mut defined_in_result);
        assert!(defined_in_result.is_some());
        let defined_in = defined_in_result.unwrap();

        let first_propery_value: Value = serde_json::from_str("{\"first\": \"this\"}").unwrap();
        let second_propery_value: Value = serde_json::from_str("{\"second\": \"that\"}").unwrap();

        let mut interface_info = InterfaceInfoImpl::new(
            2,
            id.clone(),
            Some(child_of.clone()),
            Some(defined_in.clone()),
        );
        interface_info.add_undefined_property(String::from("first"), first_propery_value.clone());
        interface_info.add_undefined_property(String::from("second"), second_propery_value.clone());

        assert!(interface_info.dtdl_version() == 2);
        assert!(interface_info.id() == &id);
        assert!(interface_info.child_of().is_some());
        assert!(interface_info.child_of().clone().unwrap() == child_of);
        assert!(interface_info.defined_in().is_some());
        assert!(interface_info.defined_in().clone().unwrap() == defined_in);
        assert!(interface_info.entity_kind() == EntityKind::Interface);
        assert!(interface_info.undefined_properties().len() == 2);
        assert!(
            interface_info.undefined_properties().get("first").unwrap().clone() == first_propery_value
        );
        assert!(
            interface_info.undefined_properties().get("second").unwrap().clone()
                == second_propery_value
        );  
    }
}
