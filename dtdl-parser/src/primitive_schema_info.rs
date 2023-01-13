// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT license.

use crate::entity_info::EntityInfo;
use crate::schema_info::SchemaInfo;

pub trait PrimitiveSchemaInfo : SchemaInfo {
    fn as_entity_info(&self) -> &dyn EntityInfo;  
}