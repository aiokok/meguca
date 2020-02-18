use super::util;
use protocol::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::str;
use yew::agent::{AgentLink, Context, HandlerId};
use yew::services::fetch;

// Key used to store AuthKey in local storage
const AUTH_KEY: &str = "auth_key";

// Global user-set options
#[derive(Serialize, Deserialize)]
pub struct Options {
	pub forced_anonymity: bool,
	pub relative_timestamps: bool,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			forced_anonymity: false,
			relative_timestamps: true,
		}
	}
}

// Stored separately from the agent to avoid needless serialization on change
// propagation. The entire application has read-only access to this singleton.
// Writes have to be coordinated through the agent to ensure propagation.
#[derive(Default)]
pub struct State {
	// Currently subscribed to thread or 0  (global thread index)
	pub feed: u64,

	// All registered threads
	pub threads: HashMap<u64, Thread>,

	// All registered posts from any sources
	pub posts: HashMap<u64, Post>,

	// Map for quick lookup of post IDs by thread
	pub posts_by_thread: SetMap<u64, u64>,

	// Authentication key
	pub auth_key: AuthKey,

	// Global user-set options
	pub options: Options,

	// Posts this user has made
	// TODO: Menu option to mark any post as mine
	// TODO: Persistance to indexedDB
	pub mine: HashSet<u64>,
}

impl State {
	fn insert_post(&mut self, p: Post) {
		self.posts_by_thread.insert(p.thread, p.id);
		self.posts.insert(p.id, p);
	}
}

super::gen_global! {pub, State, get, get_mut}

// Thread information container
#[derive(Serialize, Deserialize, Debug)]
pub struct Thread {
	pub id: u64,
	pub page: u32,

	pub subject: String,
	pub tags: Vec<String>,

	pub bumped_on: u32,
	pub created_on: u32,
	pub post_count: u64,
	pub image_count: u64,
}

// Post data
#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
	pub id: u64,
	pub page: u32,
	pub thread: u64,

	pub created_on: u32,
	pub open: bool,

	pub name: Option<String>,
	pub trip: Option<String>,
	pub flag: Option<String>,
	pub sage: bool,

	pub body: Option<post_body::Node>,
	pub image: Option<Image>,
}

// Decodes thread data received from the server as JSON
#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadDecoder {
	#[serde(flatten)]
	thread_data: Thread,

	posts: Vec<Post>,
}

// Global state storage and propagation agent
pub struct Agent {
	link: AgentLink<Self>,

	// Subscriber registry
	subscribers: DoubleSetMap<Subscription, HandlerId>,

	fetch_task: Option<yew::services::fetch::FetchTask>,
}

// Value changes to subscribe to
#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum Subscription {
	FeedID,
	AuthKey,

	// Subscribe to any changes to a post
	PostChange(u64),

	// Subscribe to thread data changes, excluding the post content level.
	// This includes changes to the post set of threads.
	ThreadChange(u64),

	// Subscribe to changes of the list of threads
	ThreadListChange,

	// Change to any field of Options
	OptionsChange,
}

#[derive(Serialize, Deserialize)]
pub enum Request {
	// Subscribe to updates of a value type
	Subscribe(Subscription),

	// Set the client authorization key
	SetAuthKey(AuthKey),

	// Fetch feed data and optionally issue a sync to it though websockets after
	// a successful fetch
	FetchFeed { id: u64, sync: bool },
}

pub enum Message {
	FetchedThreadIndex {
		data: Vec<ThreadDecoder>,
		sync: bool,
	},
	FetchFailed(String),
}

impl yew::agent::Agent for Agent {
	type Reach = Context;
	type Message = Message;
	type Input = Request;
	type Output = Subscription;

	fn create(link: AgentLink<Self>) -> Self {
		Self {
			link,
			subscribers: DoubleSetMap::default(),
			fetch_task: None,
		}
	}

	fn update(&mut self, msg: Self::Message) {
		match msg {
			Message::FetchedThreadIndex { data, sync } => {
				debug_log!("fetched", data);

				let s = get_mut();
				for t in data {
					for p in t.posts {
						self.send_change(Subscription::PostChange(p.id));
						s.insert_post(p);
					}
					self.send_change(Subscription::ThreadChange(
						t.thread_data.id,
					));
					s.threads.insert(t.thread_data.id, t.thread_data);
				}
				self.send_change(Subscription::ThreadListChange);
				self.fetch_task = None;

				if sync {
					todo!("sync to new feed")
				}
			}
			Message::FetchFailed(s) => {
				util::log_error(&s);
				util::alert(&s);
				self.fetch_task = None;
			}
		}
	}

	fn handle_input(&mut self, req: Self::Input, id: HandlerId) {
		use yew::format::{Json, Nothing};

		match req {
			Request::Subscribe(t) => {
				self.subscribers.insert(t, id);
			}
			Request::SetAuthKey(key) => {
				get_mut().auth_key = key;
				self.send_change(Subscription::AuthKey);
			}
			Request::FetchFeed { id, sync } => match id {
				0 => {
					self.fetch_task = fetch::FetchService::new()
						.fetch(
							fetch::Request::get("/api/json/index")
								.body(Nothing)
								.unwrap(),
							self.link.callback(
								move |res: fetch::Response<
									Json<
										Result<
											Vec<ThreadDecoder>,
											failure::Error,
										>,
									>,
								>| {
									let (h, body) = res.into_parts();
									match body {
										Json(Ok(body)) => {
											Message::FetchedThreadIndex {
												data: body,
												sync,
											}
										}
										_ => Message::FetchFailed(format!(
											concat!(
												"error fetching thread index: ",
												"{} {:?}"
											),
											h.status, body,
										)),
									}
								},
							),
						)
						.into();
				}
				_ => todo!("fetch thread"),
			},
		};
	}

	fn disconnected(&mut self, id: HandlerId) {
		self.subscribers.remove_by_value(&id);
	}
}

impl Agent {
	// Send change notification to all subscribers of sub
	fn send_change(&self, sub: Subscription) {
		if let Some(subs) = self.subscribers.get_by_key(&sub) {
			for id in subs.iter() {
				self.link.respond(*id, sub.clone());
			}
		}
	}
}

fn write_auth_key(key: &mut AuthKey) {
	let mut buf: [u8; 88] =
		unsafe { std::mem::MaybeUninit::uninit().assume_init() };
	base64::encode_config_slice(key, base64::STANDARD, &mut buf);

	util::with_logging(|| {
		util::local_storage()
			.set_item(AUTH_KEY, unsafe { str::from_utf8_unchecked(&buf) })
			.map_err(|e| e.into())
	});
}

// Initialize application state
pub fn init() {
	fn create_auth_key() -> AuthKey {
		let mut key = AuthKey::default();
		util::window()
			.crypto()
			.unwrap()
			.get_random_values_with_u8_array(key.as_mut())
			.unwrap();
		write_auth_key(&mut key);
		key
	}

	let mut s = get_mut();
	s.feed = util::window()
		.location()
		.hash()
		.unwrap()
		.parse()
		.unwrap_or(0);

	// Read saved or generate a new authentication key
	let ls = util::local_storage();
	s.auth_key = match ls.get_item(AUTH_KEY).unwrap() {
		Some(v) => {
			let mut key = AuthKey::default();
			match base64::decode_config_slice(
				&v,
				base64::STANDARD,
				key.as_mut(),
			) {
				Ok(_) => key,
				_ => create_auth_key(),
			}
		}
		None => create_auth_key(),
	};
}
