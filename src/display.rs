use std::fmt;

use crate::{ImageName, ImageVersion};

impl fmt::Display for ImageName  {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			let mut registry = String::new();
			if let Some(host) = &self.host {
					registry.push_str(host);
					if self.port.is_some() {
							registry.push_str(":");
							registry.push_str(self.port.unwrap().to_string().as_str());
					}
					registry.push_str("/");
			}
			let mut version = String::new();
			if let Some(v) = &self.version {
					match v {
							ImageVersion::Tag(tag) => {
									version.push_str(":");
									version.push_str(tag);
							},
							ImageVersion::Digest(digest) => {
									version.push_str("@");
									version.push_str(digest);
							},
					}
			}
			write!(f, "{registry}{}{version}", self.path)
	}
}
