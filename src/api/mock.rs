use crate::api::{Method, Regex};
use crate::server::data::{
    MockDefinition, MockMatcherClosure, MockServerHttpResponse, Pattern,
    RequestRequirements,
};
use crate::util::Join;
use crate::MockServer;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::str::FromStr;

/// Represents the primary interface to the mock server.
///
/// # Example
/// ```rust
/// extern crate httpmock;
///
/// use httpmock::{mock, with_mock_server};
/// use httpmock::Method::GET;
///
/// #[test]
/// #[with_mock_server]
/// fn simple_test() {
///    let search_mock = mock(GET, "/health")
///       .return_status(200)
///       .create();
///
///    // Act (simulates your code)
///    let response = reqwest::get("http://localhost:5000/health").unwrap();
///
///    // Make some assertions
///    assert_eq!(response.status(), 200);
///    assert_eq!(search_mock.times_called().unwrap(), 1);
/// }
/// ```
/// To be able to create a mock, you need to mark your test function with the
/// [httpmock::with_mock_server](../httpmock/attr.with_mock_server.html) attribute. If you try to
/// create a mock by calling [Mock::create](struct.Mock.html#method.create) without marking your
/// test function with [httpmock::with_mock_server](../httpmock/attr.with_mock_server.html),
/// you will receive a panic during runtime telling you about this fact.
///
/// Note that you need to call the [Mock::create](struct.Mock.html#method.create) method once you
/// are finished configuring your mock. This will create the mock on the server. Thereafter, the
/// mock will be served whenever clients send HTTP requests that match all requirements of your mock.
///
/// The [Mock::create](struct.Mock.html#method.create) method returns a reference that
/// identifies the mock at the server side. The reference can be used to fetch
/// mock related information from the server, such as the number of times it has been called or to
/// explicitly delete the mock from the server.
///
/// While [httpmock::mock](struct.Mock.html#method.create) is a convenience function, you can
/// have more control over matching the path by directly creating a new [Mock](struct.Mock.html)
/// object yourself using the [Mock::new](struct.Mock.html#method.new) method.
/// # Example
/// ```rust
/// extern crate httpmock;
///
/// use httpmock::Method::POST;
/// use httpmock::{Mock, Regex, with_mock_server};
/// use httpmock::remote::Mock;
/// use regex::Regex;
///
/// #[test]
/// #[with_mock_server]
/// fn simple_test() {
///     Mock::new()
///       .expect_path("/test")
///       .expect_path_contains("test")
///       .expect_path_matches(Regex::new(r#"test"#).unwrap())
///       .expect_method(POST)
///       .return_status(200)
///       .create();
/// }
/// ```
/// Fore more examples, please refer to
/// [this crates test directory](https://github.com/alexliesenfeld/httpmock/blob/master/tests/integration_tests.rs ).
pub struct Mock {
    mock: MockDefinition,
}

pub struct MockRef<'a> {
    id: usize,
    mock_server: &'a MockServer,
}

// TODO: implement Drop to delete the mock whenever this mock ref goes out of scope.
// This is useful whenever a user wants to explicityl delete the mock by using drop(ref)
// A problem was though that if a test is shutdown because of a panic (drop will be called)
// and this "delete" operation also panics, the user gets an awkward message
// "thread panicked while panicking. aborting."
impl<'a> MockRef<'a> {
    /// This method returns the number of times a mock has been called at the mock server.
    ///
    /// # Panics
    /// This method will panic if there is a problem to communicate with the server.
    pub fn times_called(&self) -> usize {
        self.times_called_async().join()
    }

    /// This method returns the number of times a mock has been called at the mock server.
    ///
    /// # Panics
    /// This method will panic if there is a problem to communicate with the server.
    pub async fn times_called_async(&self) -> usize {
        let response = self
            .mock_server
            .server_adapter
            .as_ref()
            .unwrap()
            .fetch_mock(self.id)
            .await
            .expect("cannot deserialize mock server response");

        response.call_counter
    }

    /// Deletes this mock from the mock server.
    ///
    /// # Panics
    /// This method will panic if there is a problem to communicate with the server.
    pub fn delete(&mut self) {
        self.delete_async().join();
    }

    pub async fn delete_async(&self) {
        self.mock_server
            .server_adapter
            .as_ref()
            .unwrap()
            .delete_mock(self.id)
            .await
            .expect("could not delete mock from server");
    }

    /// Returns the address of the mock server this mock is using. By default this is
    /// "localhost:5000" if not set otherwise by the environment variables  HTTPMOCK_HOST and
    /// HTTPMOCK_PORT.
    pub fn server_address(&self) -> &SocketAddr {
        self.mock_server.server_adapter.as_ref().unwrap().address()
    }
}

// TODO: Add possibility to limit mock server count (ulimit)
// TODO: Add matching a mock a few times and then not (countdown). Each mock request counts down 1.
// Add the following matchers that are able to extract the following info from Content-Type (potentially containing encoding, etc.)
// TODO: - add Content Type matcher that is able to determine if body is an XML type
// TODO: - add Content Type matcher that is able to determine if body is an JSON type
// TODO: - add Content Type matcher that is able to determine if body is an HTML type
// TODO: - add Content Type matcher that is able to determine if body is an text/plain type
// TODO: - add Content Type matcher that is able to determine if body is multipart form data ("multipart/form-data")
// TODO: - add Content Type matcher that is able to determine if body is "application/x-www-form-urlencoded"
// something like expect_content_type(ContentType::XML)
// TODO: Add HTTPS support and add matching the scheme
// TODO: like expect_json_body(struct) but for XML ?
// Add matchers for the following info:
// TODO: - CompressionSchemes (gzip)
// TODO: // MatchHost matches the HTTP host header field of the given request
// TODO: Return bytes from mock as response body
// TODO: Expect / return files
// TODO: XPATH/JSONPATH
// TODO: Cookies match
// TODO: Series / Statefulnes simulation
// TODO: Find the request with the most matches and show a diff on debug
// TODO: Add redirect support
impl Mock {
    /// Creates a new mock that automatically returns HTTP status code 200 if hit by an HTTP call.
    pub fn new() -> Self {
        Mock {
            mock: MockDefinition {
                request: RequestRequirements {
                    method: None,
                    path: None,
                    path_contains: None,
                    headers: None,
                    header_exists: None,
                    body: None,
                    json_body: None,
                    json_body_includes: None,
                    body_contains: None,
                    path_matches: None,
                    body_matches: None,
                    query_param_exists: None,
                    query_param: None,
                    matchers: None,
                },
                response: MockServerHttpResponse {
                    status: 200,
                    headers: None,
                    body: None,
                },
            },
        }
    }

    /// Sets the expected path. If the path of an HTTP request at the server is equal to the
    /// provided path, the request will be considered a match for this mock to respond (given all
    /// other criteria are met).
    /// * `path` - The exact path to match against.
    pub fn expect_path(mut self, path: &str) -> Self {
        self.mock.request.path = Some(path.to_string());
        self
    }

    /// Sets an expected path substring. If the path of an HTTP request at the server contains t,
    /// his substring the request will be considered a match for this mock to respond (given all
    /// other criteria are met).
    /// * `substring` - The substring to match against.
    pub fn expect_path_contains(mut self, substring: &str) -> Self {
        if self.mock.request.path_contains.is_none() {
            self.mock.request.path_contains = Some(Vec::new());
        }

        self.mock
            .request
            .path_contains
            .as_mut()
            .unwrap()
            .push(substring.to_string());

        self
    }

    /// Sets an expected path regex. If the path of an HTTP request at the server matches this,
    /// regex the request will be considered a match for this mock to respond (given all other
    /// criteria are met).
    /// * `regex` - The regex to match against.
    pub fn expect_path_matches(mut self, regex: Regex) -> Self {
        if self.mock.request.path_matches.is_none() {
            self.mock.request.path_matches = Some(Vec::new());
        }

        self.mock
            .request
            .path_matches
            .as_mut()
            .unwrap()
            .push(Pattern::from_regex(regex));
        self
    }

    /// Sets the expected HTTP method. If the path of an HTTP request at the server matches this regex,
    /// the request will be considered a match for this mock to respond (given all other
    /// criteria are met).
    /// * `method` - The HTTP method to match against.
    pub fn expect_method(mut self, method: Method) -> Self {
        self.mock.request.method = Some(method.to_string());
        self
    }

    /// Sets an expected HTTP header. If one of the headers of an HTTP request at the server matches
    /// the provided header key and value, the request will be considered a match for this mock to
    /// respond (given all other criteria are met).
    ///
    /// * `name` - The HTTP header name (header names are case-insensitive by RFC 2616).
    /// * `value` - The HTTP header value.
    pub fn expect_header(mut self, name: &str, value: &str) -> Self {
        if self.mock.request.headers.is_none() {
            self.mock.request.headers = Some(BTreeMap::new());
        }

        self.mock
            .request
            .headers
            .as_mut()
            .unwrap()
            .insert(name.to_string(), value.to_string());

        self
    }

    /// Sets an expected HTTP header to exists. If one of the headers of an HTTP request at the
    /// server matches the provided header name, the request will be considered a match for this
    /// mock to respond (given all other criteria are met).
    ///
    /// * `name` - The HTTP header name (header names are case-insensitive by RFC 2616).
    pub fn expect_header_exists(mut self, name: &str) -> Self {
        if self.mock.request.header_exists.is_none() {
            self.mock.request.header_exists = Some(Vec::new());
        }

        self.mock
            .request
            .header_exists
            .as_mut()
            .unwrap()
            .push(name.to_string());
        self
    }

    /// Sets the expected HTTP body. If the body of an HTTP request at the server matches the
    /// provided body, the request will be considered a match for this mock to respond
    /// (given all other criteria are met). This is an exact match, so all characters are taken
    /// into account, such as whitespace, tabs, etc.
    ///  * `contents` - The HTTP body to match against.
    pub fn expect_body(mut self, contents: &str) -> Self {
        self.mock.request.body = Some(contents.to_string());
        self
    }

    /// Sets the expected HTTP body JSON string. This method expects a serializable serde object
    /// that will be parsed into JSON. If the body of an HTTP request at the server matches the
    /// body according to the provided JSON object, the request will be considered a match for
    /// this mock to respond (given all other criteria are met).
    ///
    /// This is an exact match, so all characters are taken into account at the server side.
    ///
    /// The provided JSON object needs to be both, a deserializable and
    /// serializable serde object. Note that this method does not set the "Content-Type" header
    /// automatically, so you need to provide one yourself!
    ///
    /// * `body` - The HTTP body object that will be serialized to JSON using serde.
    pub fn expect_json_body<T>(mut self, body: &T) -> Self
    where
        T: Serialize,
    {
        let serialized_body =
            serde_json::to_string(body).expect("cannot serialize json body to JSON string ");

        let value =
            Value::from_str(&serialized_body).expect("cannot convert JSON string to serde value");

        self.mock.request.json_body = Some(value);
        self
    }

    /// Sets an expected partial HTTP body JSON string.
    ///
    /// If the body of an HTTP request at the server matches the
    /// partial, the request will be considered a match for
    /// this mock to respond (given all other criteria are met).
    ///
    /// # Important Notice
    /// The partial string needs to contain the full JSON object path from the root.
    ///
    /// ## Example
    /// If your application sends the following JSON request data to the mock server
    /// ```json
    /// {
    ///     "parent_attribute" : "Some parent data goes here",
    ///     "child" : {
    ///         "target_attribute" : "Target value",
    ///         "other_attribute" : "Another value"
    ///     }
    /// }
    /// ```
    /// and you only want to make sure that `target_attribute` has the value
    /// `Target value`, you need to provide a partial JSON string to this method, that starts from
    /// the root of the JSON object, but may leave out unimportant values:
    /// ```
    /// use httpmock::Method::POST;
    ///
    /// #[test]
    /// #[with_mock_server]
    /// fn partial_json_test() {
    ///     mock(POST, "/path")
    ///         .expect_json_body_partial(r#"
    ///             {
    ///                 "child" : {
    ///                     "target_attribute" : "Target value"
    ///                 }
    ///             }
    ///         "#)
    ///         .return_status(200)
    ///         .create();
    /// }
    ///
    /// ```
    /// String format and attribute order will be ignored.
    ///
    /// * `partial` - The JSON partial.
    pub fn expect_json_body_partial(mut self, partial: &str) -> Self {
        if self.mock.request.json_body_includes.is_none() {
            self.mock.request.json_body_includes = Some(Vec::new());
        }

        let value = Value::from_str(partial).expect("cannot convert JSON string to serde value");

        self.mock
            .request
            .json_body_includes
            .as_mut()
            .unwrap()
            .push(value);
        self
    }

    /// Sets an expected HTTP body substring. If the body of an HTTP request at the server contains
    /// the provided substring, the request will be considered a match for this mock to respond
    /// (given all other criteria are met).
    /// * `substring` - The substring that will matched against.
    pub fn expect_body_contains(mut self, substring: &str) -> Self {
        if self.mock.request.body_contains.is_none() {
            self.mock.request.body_contains = Some(Vec::new());
        }

        self.mock
            .request
            .body_contains
            .as_mut()
            .unwrap()
            .push(substring.to_string());
        self
    }

    /// Sets an expected HTTP body regex. If the body of an HTTP request at the server matches
    /// the provided regex, the request will be considered a match for this mock to respond
    /// (given all other criteria are met).
    /// * `regex` - The regex that will matched against.
    pub fn expect_body_matches(mut self, regex: Regex) -> Self {
        if self.mock.request.body_matches.is_none() {
            self.mock.request.body_matches = Some(Vec::new());
        }

        self.mock
            .request
            .body_matches
            .as_mut()
            .unwrap()
            .push(Pattern::from_regex(regex));
        self
    }

    /// Sets an expected query parameter. If the query parameters of an HTTP request at the server
    /// contains the provided query parameter name and value, the request will be considered a
    /// match for this mock to respond (given all other criteria are met).
    /// * `name` - The query parameter name that will matched against.
    /// * `value` - The value parameter name that will matched against.
    pub fn expect_query_param(mut self, name: &str, value: &str) -> Self {
        if self.mock.request.query_param.is_none() {
            self.mock.request.query_param = Some(BTreeMap::new());
        }

        self.mock
            .request
            .query_param
            .as_mut()
            .unwrap()
            .insert(name.to_string(), value.to_string());

        self
    }

    /// Sets an expected query parameter name. If the query parameters of an HTTP request at the server
    /// contains the provided query parameter name (not considering the value), the request will be
    /// considered a match for this mock to respond (given all other criteria are met).
    /// * `name` - The query parameter name that will matched against.
    pub fn expect_query_param_exists(mut self, name: &str) -> Self {
        if self.mock.request.query_param_exists.is_none() {
            self.mock.request.query_param_exists = Some(Vec::new());
        }

        self.mock
            .request
            .query_param_exists
            .as_mut()
            .unwrap()
            .push(name.to_string());

        self
    }

    /// Sets the HTTP status that the mock will return, if an HTTP request fulfills all of
    /// the mocks requirements.
    /// * `status` - The HTTP status that the mock server will return.
    pub fn return_status(mut self, status: usize) -> Self {
        self.mock.response.status = status as u16;
        self
    }

    /// Sets the HTTP response body that the mock will return, if an HTTP request fulfills all of
    /// the mocks requirements.
    /// * `body` - The HTTP response body that the mock server will return.
    pub fn return_body(mut self, body: &str) -> Self {
        self.mock.response.body = Some(body.to_string());
        self
    }

    /// Sets the HTTP response JSON body that the mock will return, if an HTTP request fulfills all of
    /// the mocks requirements.
    ///
    /// The provided JSON object needs to be both, a deserializable and
    /// serializable serde object. Note that this method does not set the "Content-Type" header
    /// automatically, so you need to provide one yourself!
    ///
    /// * `body` - The HTTP response body the mock server will return in the form of a JSON string.
    pub fn return_json_body<T>(mut self, body: &T) -> Self
    where
        T: Serialize,
    {
        let serialized_body =
            serde_json::to_string(body).expect("cannot serialize json body to JSON string ");
        self.mock.response.body = Some(serialized_body);
        self
    }

    /// Sets an HTTP header that the mock will return, if an HTTP request fulfills all of
    /// the mocks requirements.
    /// * `name` - The name of the header.
    /// * `value` - The value of the header.
    pub fn return_header(mut self, name: &str, value: &str) -> Self {
        if self.mock.response.headers.is_none() {
            self.mock.response.headers = Some(BTreeMap::new());
        }

        self.mock
            .response
            .headers
            .as_mut()
            .unwrap()
            .insert(name.to_string(), value.to_string());

        self
    }

    /// This method creates the mock at the server side and returns a `Mock` object
    /// representing the reference of the created mock at the server.
    ///
    /// # Panics
    /// This method will panic if your test method was not marked using the the
    /// `httpmock::with_mock_server` annotation.
    pub async fn create_on_async<'a>(self, mock_server: &'a MockServer) -> MockRef<'a> {
        let response = mock_server
            .server_adapter
            .as_ref()
            .unwrap()
            .create_mock(&self.mock)
            .await
            .expect("Cannot deserialize mock server response");
        MockRef {
            id: response.mock_id,
            mock_server,
        }
    }

    /// This method creates the mock at the server side and returns a `Mock` object
    /// representing the reference of the created mock at the server.
    ///
    /// # Panics
    /// This method will panic if your test method was not marked using the the
    /// `httpmock::with_mock_server` annotation.
    pub fn create_on<'a>(self, mock_server: &'a MockServer) -> MockRef<'a> {
        self.create_on_async(mock_server).join()
    }

    pub fn expect_match(mut self, request_matcher: MockMatcherClosure) -> Self {
        if self.mock.request.matchers.is_none() {
            self.mock.request.matchers = Some(Vec::new());
        }

        self.mock
            .request
            .matchers
            .as_mut()
            .unwrap()
            .push(request_matcher);

        self
    }
}

impl Default for Mock {
    fn default() -> Self {
        Self::new()
    }
}
