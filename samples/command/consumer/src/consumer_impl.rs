// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT license.

use log::info;
use proto::consumer::consumer_server::Consumer;
use proto::consumer::{PublishRequest, PublishResponse};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct ConsumerImpl {}

#[tonic::async_trait]
impl Consumer for ConsumerImpl {
    /// Publish implementation.
    ///
    /// # Arguments
    /// * `request` - Publish request.
    async fn publish(
        &self,
        request: Request<PublishRequest>,
    ) -> Result<Response<PublishResponse>, Status> {
        let request_inner = request.into_inner();

        info!(
            "Received a publish for id {} with the value {}",
            request_inner.id, request_inner.value
        );

        let response = PublishResponse {};

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod consumer_impl_tests {
    use super::*;
    use async_std::task;

    #[test]
    fn publish_test() {
        let consumer_impl = ConsumerImpl {};

        let id = String::from("some-id");
        let value = String::from("some-value");

        let request = tonic::Request::new(PublishRequest { id, value });
        let result = task::block_on(consumer_impl.publish(request));
        assert!(result.is_ok());
    }
}
