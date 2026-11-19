/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::any::type_name;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::USER_AGENT;
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use serde_json::{Value, from_value};

use crate::e621::io::{Login, emergency_exit};
use crate::e621::sender::entries::{
    AliasEntry, BulkPostEntry, PostEntry, TagEntry, UserEntry, V2PostEntry,
};

pub(crate) mod entries;

/// Creates a hashmap through similar syntax of the `vec` macro.
///
/// # Arguments
/// * `x`: Represents multiple tuples being passed as parameter.
///
/// # Example
///
/// ```rust
/// # use std::collections::hashmap;
///
/// let hashmap = hashmap![("Testing", "testing"), ("Example", "example")];
///
/// assert_eq!(hashmap["Testing"], String::from("testing"));
/// ```
#[macro_export]
macro_rules! hashmap {
    ( $( $x:expr ),* ) => {
        {
            let mut hash_map: HashMap<String, String> = HashMap::new();
            $(
                let (calling_name, url) = $x;
                hash_map.insert(String::from(calling_name), String::from(url));
            )*

            hash_map
        }
    };
}

/// Default user agent value.
const USER_AGENT_VALUE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (by ",
    env!("CARGO_PKG_AUTHORS"),
    " on e621)"
);

/// A reference counted client used for all searches by the [Grabber], [Blacklist], [`E621WebConnector`], etc.
struct SenderClient {
    /// [Client] wrapped in a [Rc] so only one instance of the client exists. This will prevent an overabundance of
    /// clients in the code.
    client: Rc<Client>,
    /// The username and API key used for HTTP Basic authentication.
    auth: Rc<Option<(String, String)>>,
}

impl SenderClient {
    /// Creates root client.
    fn new(auth: Option<(String, String)>) -> Self {
        trace!("SenderClient initializing with USER_AGENT_VALUE \"{USER_AGENT_VALUE}\"");

        SenderClient {
            client: Rc::new(SenderClient::build_client()),
            auth: Rc::new(auth),
        }
    }

    /// Runs client through a builder to give it required settings.
    /// Cookies aren't stored in the client, `TCP_NODELAY` is on, and timeout is changed from 30 seconds to 60.
    fn build_client() -> Client {
        Client::builder()
            .use_rustls_tls()
            .tcp_keepalive(Duration::from_secs(30))
            .tcp_nodelay(true)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    /// A wrapping function that acts the exact same as `self.client.get` but will instead attach the user agent header
    /// before returning the [`RequestBuilder`]. This will ensure that all requests sent have the proper user agent info.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to request.
    ///
    /// returns: `RequestBuilder`
    pub(crate) fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url).header(USER_AGENT, USER_AGENT_VALUE)
    }

    /// This is the same as `self.get(url)` but will attach the authorization header with username and API key.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to request.
    ///
    /// returns: `RequestBuilder`
    pub(crate) fn get_with_auth(&self, url: &str) -> RequestBuilder {
        match self.auth.as_ref() {
            Some((username, api_key)) => self.get(url).basic_auth(username, Some(api_key)),
            None => self.get(url),
        }
    }
}

impl Clone for SenderClient {
    /// Creates a new instance of `SenderClient`, but clones the [Rc] of the root client, ensuring that all requests are
    /// going to the same client.
    fn clone(&self) -> Self {
        SenderClient {
            client: Rc::clone(&self.client),
            auth: Rc::clone(&self.auth),
        }
    }
}

/// A sender that handles direct calls to the API.
///
/// This acts as a safety layer to ensure calls to the API are less error prone.
pub(crate) struct RequestSender {
    /// The client that will be used to send all requests.
    ///
    /// Even though the [`SenderClient`] isn't wrapped in a [Rc], the main client inside of it is, this will ensure that
    /// all request are only sent through one client.
    client: SenderClient,
    /// All available urls to use with the sender.
    urls: Rc<RefCell<HashMap<String, String>>>,
}

impl RequestSender {
    pub(crate) fn new() -> Self {
        let login = Login::get();
        let auth = if login.is_empty() {
            None
        } else {
            Some((login.username().to_string(), login.api_key().to_string()))
        };

        RequestSender {
            client: SenderClient::new(auth),
            urls: Rc::new(RefCell::new(RequestSender::initialize_url_map())),
        }
    }

    /// Initializes all the urls that will be used by the sender.
    fn initialize_url_map() -> HashMap<String, String> {
        hashmap![
            ("posts", "https://e621.net/posts.json"),
            ("pool", "https://e621.net/pools/"),
            ("set", "https://e621.net/post_sets/"),
            ("single", "https://e621.net/posts/"),
            ("tag", "https://e621.net/tags/"),
            ("tag_bulk", "https://e621.net/tags.json"),
            ("alias", "https://e621.net/tag_aliases.json"),
            ("user_me", "https://e621.net/users/me.json")
        ]
    }

    /// If the client authenticated or not.
    pub(crate) fn is_authenticated(&self) -> bool {
        self.client.auth.is_some()
    }

    /// Updates all the urls from e621 to e926.
    pub(crate) fn update_to_safe(&mut self) {
        self.urls
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, value)| *value = value.replace("e621", "e926"));
    }

    /// Outputs a transport-level request failure before exiting.
    fn output_transport_error(&self, error: &reqwest::Error) -> ! {
        error!("Error occurred from sent request. Error: {error}");
        trace!("Url where error occurred: {:#?}", error.url());

        emergency_exit("To prevent the program from crashing, it will do an emergency exit.");
        unreachable!()
    }

    /// Outputs a descriptive HTTP error from e621/e926 or Cloudflare before exiting.
    fn output_http_error(&self, status: StatusCode, url: &Url, body: Option<&str>) -> ! {
        let code = status.as_u16();
        error!("Request failed with HTTP status {status} from \"{url}\".");
        trace!("The response code from the server was: {code}");

        if let Some(details) = body.and_then(extract_api_error_details) {
            error!("Server response: {details}");
        }

        match code {
            400 => {
                error!(
                    "Bad request: the request parameters were invalid or the requested feature is unavailable."
                );
            }
            401 => {
                error!(
                    "Authentication failed: check your username and API key in login.json \
                     (the API key may be expired or revoked, or the account may be restricted)."
                );
            }
            403 => {
                error!(
                    "Access denied: insufficient permissions, private resource, or the User-Agent/client \
                     was blocked by the server."
                );
            }
            404 => {
                error!("The requested resource (post, pool, set, or user) was not found.");
            }
            410 => {
                error!(
                    "Pagination limit reached or invalid page parameter (e621 restricts numbered \
                     pagination to at most 750 pages)."
                );
            }
            413 => {
                error!("The request exceeded server size or parameter limits.");
            }
            422 => {
                error!(
                    "The server rejected the query or input (e.g., too many tags, overly complex \
                     search, search timeout, or GET parameter over 16 KB)."
                );
            }
            429 => {
                error!(
                    "Rate limited / throttled by the server. Wait a moment and try again later."
                );
            }
            500 => {
                error!(
                    "There was an internal error or database timeout on the server, \
                     please try using the downloader later until the issue is resolved."
                );
            }
            502 => {
                error!(
                    "Bad gateway: Cloudflare could not reach e621/e926. Please try again later."
                );
            }
            503 => {
                error!(
                    "Service unavailable: exceeded the 2 requests/second rate limit or e621/e926 \
                     is temporarily unavailable. Please try again later."
                );
            }
            520 | 522 | 524 | 525 => {
                error!(
                    "Cloudflare encountered a connection error reaching e621/e926 (HTTP {code}). \
                     Please try again later."
                );
            }
            _ => {
                error!("Unexpected HTTP error response (status {code}).");
            }
        }

        emergency_exit("To prevent the program from crashing, it will do an emergency exit.");
        unreachable!()
    }

    /// Gets the response from a sent request and checks to ensure it was successful.
    ///
    /// # Arguments
    ///
    /// * `result`: The result to check.
    ///
    /// returns: Response
    fn check_response(&self, result: Result<Response, reqwest::Error>) -> Response {
        match result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    response
                } else {
                    let url = response.url().clone();
                    let body = response.text().ok();
                    self.output_http_error(status, &url, body.as_deref());
                }
            }
            Err(ref error) => self.output_transport_error(error),
        }
    }

    /// Sends request to download image.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to the file to download.
    /// * `file_size`: The file size of the file.
    ///
    /// returns: Vec<u8, Global>
    pub(crate) fn download_image(&self, url: &str, file_size: i64) -> Vec<u8> {
        let mut image_response = self.check_response(self.client.get(url).send());
        let mut image_bytes: Vec<u8> =
            Vec::with_capacity(usize::try_from(file_size).expect("Something went wrong."));
        image_response
            .copy_to(&mut image_bytes)
            .with_context(|| "Failed to download image!".to_string())
            .unwrap();

        image_bytes
    }

    /// Appends base url with id/name before ending with `.json`.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to change.
    /// * `append`: The id/name.
    ///
    /// returns: String
    pub(crate) fn append_url(&self, url: &str, append: &str) -> String {
        format!("{url}{append}.json")
    }

    /// Gets entry by type `T`, this is used for every request where the url needs to be appended to.
    ///
    /// # Arguments
    ///
    /// * `id`: The id to search for.
    /// * `url_type_key`: The type of url to use.
    ///
    /// returns: T
    pub(crate) fn get_entry_from_appended_id<T>(&self, id: &str, url_type_key: &str) -> T
    where
        T: DeserializeOwned,
    {
        self.check_response(
            self.client
                .get_with_auth(&self.append_url(&self.urls.borrow()[url_type_key], id))
                .send(),
        )
        .json::<T>()
        .with_context(|| {
            format!(
                "Json was unable to deserialize to \"{}\"!\n\
                 url_type_key: {}\n\
                 id: {}",
                type_name::<T>(),
                url_type_key,
                id
            )
        })
        .unwrap()
    }

    /// Fetches the authenticated user's profile via `/users/me.json`.
    pub(crate) fn get_current_user(&self) -> UserEntry {
        self.check_response(
            self.client
                .get_with_auth(&self.urls.borrow()["user_me"])
                .send(),
        )
        .json::<UserEntry>()
        .with_context(|| {
            format!(
                "Json was unable to deserialize to \"{}\" from user_me!",
                type_name::<UserEntry>()
            )
        })
        .unwrap()
    }

    /// Performs a bulk search for posts using tags to filter the response.
    ///
    /// # Arguments
    ///
    /// * `searching_tag`: The tags for filtering.
    /// * `page`: The page specifier to search for (numbered like `"1"` or sequential like `"b12345"`).
    /// * `extended`: Whether to request categorized tags (`mode=extended`) or flat tags (`mode=basic`).
    ///
    /// returns: `BulkPostEntry`
    pub(crate) fn bulk_search(
        &self,
        searching_tag: &str,
        page: &str,
        extended: bool,
    ) -> BulkPostEntry {
        let mode = if extended { "extended" } else { "basic" };
        debug!("Downloading page {page} of tag {searching_tag} (mode={mode})");

        self.check_response(
            self.client
                .get_with_auth(&self.urls.borrow()["posts"])
                .query(&[
                    ("tags", searching_tag),
                    ("page", page),
                    ("limit", "320"),
                    ("mode", mode),
                ])
                .send(),
        )
        .json::<Vec<V2PostEntry>>()
        .with_context(|| {
            error!(
                "Unable to deserialize json to \"{}\"!",
                type_name::<Vec<PostEntry>>()
            );
            "Failed to perform bulk search...".to_string()
        })
        .map(|posts| BulkPostEntry {
            posts: posts.into_iter().map(PostEntry::from).collect(),
        })
        .unwrap()
    }

    /// Gets a single post using the v2 response and converts it to
    /// the internal post representation.
    pub(crate) fn get_post_entry(&self, id: &str, extended: bool) -> PostEntry {
        let mode = if extended { "extended" } else { "basic" };

        self.check_response(
            self.client
                .get_with_auth(&self.append_url(&self.urls.borrow()["single"], id))
                .query(&[("mode", mode)])
                .send(),
        )
        .json::<V2PostEntry>()
        .map(PostEntry::from)
        .with_context(|| format!("Unable to deserialize post response for ID {id}!"))
        .unwrap()
    }

    /// Gets tags by their name.
    ///
    /// # Arguments
    ///
    /// * `tag`: The name of the tag.
    ///
    /// returns: Vec<`TagEntry`, Global>
    pub(crate) fn get_tags_by_name(&self, tag: &str) -> Vec<TagEntry> {
        let result: Value = self
            .check_response(
                self.client
                    .get(&self.urls.borrow()["tag_bulk"])
                    .query(&[("search[name]", tag)])
                    .send(),
            )
            .json()
            .with_context(|| {
                format!(
                    "Json was unable to deserialize to \"{}\"!\n\
                     url_type_key: tag_bulk\n\
                     tag: {}",
                    type_name::<Value>(),
                    tag
                )
            })
            .unwrap();
        if result.is_object() {
            vec![]
        } else {
            from_value::<Vec<TagEntry>>(result)
                .with_context(|| {
                    error!(
                        "Unable to deserialize Value to \"{}\"!",
                        type_name::<Vec<TagEntry>>()
                    );
                    "Failed to perform bulk search...".to_string()
                })
                .unwrap()
        }
    }

    /// Queries aliases and returns response.
    ///
    /// # Arguments
    ///
    /// * `tag`: The alias to search for.
    ///
    /// returns: Option<Vec<`AliasEntry`, Global>>
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub(crate) fn query_aliases(&self, tag: &str) -> Option<Vec<AliasEntry>> {
        let result = self
            .check_response(
                self.client
                    .get(&self.urls.borrow()["alias"])
                    .query(&[
                        ("commit", "Search"),
                        ("search[name_matches]", tag),
                        ("search[order]", "status"),
                    ])
                    .send(),
            )
            .json::<Vec<AliasEntry>>();

        match result {
            Ok(e) => Some(e),
            Err(e) => {
                trace!("No alias was found for {tag}...");
                trace!("Printing trace message for why None was returned...");
                trace!("{e}");
                None
            }
        }
    }
}

impl Clone for RequestSender {
    fn clone(&self) -> Self {
        RequestSender {
            client: self.client.clone(),
            urls: Rc::clone(&self.urls),
        }
    }
}

/// Extracts the server's `message` or `reason` (and optional exception `code`) from an e621 JSON error response.
fn extract_api_error_details(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    let detail = json
        .get("message")
        .or_else(|| json.get("reason"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let log_code = json
        .get("code")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    Some(match log_code {
        Some(code) => format!("{detail} (error code: {code})"),
        None => detail.to_string(),
    })
}
