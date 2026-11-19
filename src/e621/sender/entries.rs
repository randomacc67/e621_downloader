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

use serde::{Deserialize, Serialize};

use crate::e621::io::tag::TagType;

/// GET return of alias entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AliasEntry {
    /// Alias ID.
    pub(crate) id: i64,
    /// Alias name.
    pub(crate) antecedent_name: String,
    /// Reason for the alias.
    pub(crate) reason: String,
    /// ID of the creator of the alias.
    pub(crate) creator_id: i64,
    /// The date the alias was created.
    pub(crate) created_at: Option<String>,
    /// Forum post id tied to the request for the alias to be approved.
    pub(crate) forum_post_id: Option<i64>,
    /// The date for when the alias was updated.
    pub(crate) updated_at: Option<String>,
    /// Forum topic ID for the thread where the request for alias approval was created.
    pub(crate) forum_topic_id: Option<i64>,
    /// Original tag name.
    pub(crate) consequent_name: String,
    /// Current status of the alias.
    /// Can be `approved`, `active`, `pending`, `deleted`, `retired`, `processing`, and `queued`.
    ///
    /// # Error
    /// Optionally, there can also be an `error` prompt with the following format:
    /// `"error: cannot update a new record"`
    /// ## Reason for Error
    /// This is probably an internal error with the server, and while it is exceptionally rare,
    /// there is still a probability.
    pub(crate) status: String,
    /// The amount of post the aliased tag is tied to.
    pub(crate) post_count: i64,
    /// ID of the user that approved the alias.
    pub(crate) approver_id: Option<i64>,
}

/// GET return of tag entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TagEntry {
    /// Id of the tag.
    pub(crate) id: i64,
    /// Name of the tag.
    pub(crate) name: String,
    /// Amount of posts that uses the tag.
    pub(crate) post_count: i64,
    /// Related tags that this tag is commonly paired with.
    pub(crate) related_tags: Option<String>,
    /// Most recent date the `related_tags` was updated.
    pub(crate) related_tags_updated_at: Option<String>,
    /// The type of tag it is.
    ///
    /// This tag can be the following types:
    /// - `0`: General;
    /// - `1`: Artist;
    /// - `2`: Contributor;
    /// - `3`: Copyright;
    /// - `4`: Character;
    /// - `5`: Species;
    /// - `6`: Invalid;
    /// - `7`: Meta;
    /// - `8`: Lore;
    pub(crate) category: u8,
    /// Whether or not the tag is locked.
    pub(crate) is_locked: bool,
    /// The date the tag was created.
    pub(crate) created_at: String,
    /// The date the tag was updated.
    pub(crate) updated_at: String,
}

impl TagEntry {
    /// Constrains the `TagType` enum to a tags type specifically.
    ///
    /// This can only be `TagType::General` or `TagType::Artist`.
    pub(crate) fn to_tag_type(&self) -> TagType {
        match self.category {
            1 => TagType::Artist,
            _ => TagType::General,
        }
    }
}

/// Wrapper struct that holds the return of bulk searches.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BulkPostEntry {
    /// All posts in the bulk.
    pub(crate) posts: Vec<PostEntry>,
}

/// Extended v2 post response. This is converted into [`PostEntry`] so the rest
/// of the application does not need to know about the v2 response layout.
#[derive(Debug, Deserialize)]
pub(crate) struct V2PostEntry {
    pub(crate) id: i64,
    pub(crate) created_at: String,
    pub(crate) updated_at: Option<String>,
    pub(crate) change_seq: i64,
    pub(crate) files: V2Files,
    pub(crate) uploader_id: i64,
    pub(crate) uploader_name: Option<String>,
    pub(crate) approver_id: Option<i64>,
    pub(crate) stats: V2Stats,
    pub(crate) flags: Flags,
    pub(crate) has: V2Has,
    pub(crate) relationships: V2Relationships,
    pub(crate) pools: Vec<i64>,
    pub(crate) rating: String,
    pub(crate) locked_tags: Vec<String>,
    pub(crate) sources: Vec<String>,
    pub(crate) description: String,
    pub(crate) tags: V2Tags,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Files {
    pub(crate) meta: V2FileMeta,
    pub(crate) original: V2Original,
    pub(crate) preview: V2Preview,
    pub(crate) sample: V2Sample,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2FileMeta {
    pub(crate) md5: Option<String>,
    pub(crate) ext: Option<String>,
    pub(crate) size: i64,
    pub(crate) has_sample: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Original {
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Preview {
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) jpg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Sample {
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) jpg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Stats {
    pub(crate) score: Score,
    pub(crate) fav_count: i64,
    pub(crate) is_favorited: bool,
    pub(crate) comment_count: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Has {
    pub(crate) children: bool,
    pub(crate) active_children: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2Relationships {
    pub(crate) parent_id: Option<i64>,
    pub(crate) children: Vec<i64>,
}

/// Tag payload from the v2 posts API: a flat array in `mode=basic` or categorized arrays in `mode=extended`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum V2Tags {
    Basic(Vec<String>),
    Extended(V2CategorizedTags),
}

#[derive(Debug, Deserialize)]
pub(crate) struct V2CategorizedTags {
    pub(crate) general: Vec<String>,
    pub(crate) species: Vec<String>,
    pub(crate) character: Vec<String>,
    pub(crate) copyright: Vec<String>,
    pub(crate) artist: Vec<String>,
    pub(crate) contributor: Vec<String>,
    pub(crate) invalid: Vec<String>,
    pub(crate) lore: Vec<String>,
    pub(crate) meta: Vec<String>,
}

impl From<V2PostEntry> for PostEntry {
    fn from(post: V2PostEntry) -> Self {
        let V2PostEntry {
            id,
            created_at,
            updated_at,
            change_seq,
            files,
            uploader_id,
            uploader_name,
            approver_id,
            stats,
            flags,
            has,
            relationships,
            pools,
            rating,
            locked_tags,
            sources,
            description,
            tags,
        } = post;

        PostEntry {
            id,
            created_at,
            updated_at,
            file: File {
                width: files.original.width,
                height: files.original.height,
                ext: files.meta.ext.unwrap_or_default(),
                size: files.meta.size,
                md5: files.meta.md5.unwrap_or_default(),
                url: files.original.url,
            },
            preview: Preview {
                width: files.preview.width,
                height: files.preview.height,
                url: files.preview.jpg,
            },
            sample: Sample {
                has: files.meta.has_sample,
                width: files.sample.width,
                height: files.sample.height,
                url: files.sample.jpg,
            },
            score: stats.score,
            tags: match tags {
                V2Tags::Basic(flat_tags) => Tags {
                    general: flat_tags,
                    ..Default::default()
                },
                V2Tags::Extended(categorized) => Tags {
                    general: categorized.general,
                    species: categorized.species,
                    character: categorized.character,
                    copyright: categorized.copyright,
                    artist: categorized.artist,
                    contributor: categorized.contributor,
                    invalid: categorized.invalid,
                    lore: categorized.lore,
                    meta: categorized.meta,
                },
            },
            locked_tags,
            change_seq,
            flags,
            rating,
            fav_count: stats.fav_count,
            sources,
            pools,
            relationships: Relationships {
                parent_id: relationships.parent_id,
                has_children: has.children,
                has_active_children: has.active_children,
                children: relationships.children,
            },
            approver_id,
            uploader_id,
            uploader_name,
            description,
            comment_count: stats.comment_count,
            is_favorited: stats.is_favorited,
        }
    }
}

/// GET return of post entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PostEntry {
    /// The ID number of the post.
    pub(crate) id: i64,
    /// The time the post was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    ///  The time the post was last updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: Option<String>,
    /// The main image of the post.
    pub(crate) file: File,
    /// The preview image of the post.
    pub(crate) preview: Preview,
    /// The sample image of the post.
    pub(crate) sample: Sample,
    /// The score of the post.
    pub(crate) score: Score,
    /// The tags tied to the post.
    pub(crate) tags: Tags,
    /// An array of tags that are locked on the post.
    pub(crate) locked_tags: Vec<String>,
    /// An ID that increases for every post alteration on E6 (explained below)
    ///
    /// `change_seq` is a number that is increased every time a post is changed on the site.
    /// It gets updated whenever a post has any of these values change:
    ///
    /// - `tag_string`
    /// - `source`
    /// - `description`
    /// - `rating`
    /// - `md5`
    /// - `parent_id`
    /// - `approver_id`
    /// - `is_deleted`
    /// - `is_pending`
    /// - `is_flagged`
    /// - `is_rating_locked`
    /// - `is_pending`
    /// - `is_flagged`
    /// - `is_rating_locked`
    pub(crate) change_seq: i64,
    /// All the flags that could be raised on the post.
    pub(crate) flags: Flags,
    /// The post’s rating. Either `s`, `q` or `e`.
    pub(crate) rating: String,
    /// How many people have favorited the post.
    pub(crate) fav_count: i64,
    /// The source field of the post.
    pub(crate) sources: Vec<String>,
    /// An array of Pool IDs that the post is a part of.
    pub(crate) pools: Vec<i64>,
    /// The relationships of the post.
    pub(crate) relationships: Relationships,
    /// The ID of the user that approved the post, if available.
    pub(crate) approver_id: Option<i64>,
    /// The ID of the user that uploaded the post.
    pub(crate) uploader_id: i64,
    /// The username of the user that uploaded the post, if available.
    pub(crate) uploader_name: Option<String>,
    /// The post’s description.
    pub(crate) description: String,
    /// The count of comments on the post.
    pub(crate) comment_count: i64,
    /// If provided auth credentials, will return if the authenticated user has favorited the post or not.
    /// HTTP Basic Auth is recommended over `login` and `api_key` parameters in the URL.
    pub(crate) is_favorited: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct File {
    /// The width of the post.
    pub(crate) width: i64,
    /// The height of the post.
    pub(crate) height: i64,
    /// The file’s extension.
    pub(crate) ext: String,
    /// The size of the file in bytes.
    pub(crate) size: i64,
    /// The md5 of the file.
    pub(crate) md5: String,
    /// The URL where the file is hosted on E6
    pub(crate) url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Preview {
    /// The width of the post preview.
    pub(crate) width: i64,
    /// The height of the post preview.
    pub(crate) height: i64,
    /// The URL where the preview file is hosted on E6
    pub(crate) url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Sample {
    ///  If the post has a sample/thumbnail or not.
    pub(crate) has: Option<bool>,
    /// The width of the post sample.
    pub(crate) height: i64,
    /// The height of the post sample.
    pub(crate) width: i64,
    /// The URL where the sample file is hosted on E6.
    pub(crate) url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Score {
    /// The number of times voted up.
    pub(crate) up: i64,
    /// A negative number representing the number of times voted down.
    pub(crate) down: i64,
    /// The total score (up + down).
    pub(crate) total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Tags {
    /// An array of all the `general` tags on the post.
    pub(crate) general: Vec<String>,
    /// An array of all the `species` tags on the post.
    pub(crate) species: Vec<String>,
    /// An array of all the `character` tags on the post.
    pub(crate) character: Vec<String>,
    /// An array of all the `copyright` tags on the post.
    pub(crate) copyright: Vec<String>,
    /// An array of all the `artist` tags on the post.
    pub(crate) artist: Vec<String>,
    /// An array of all the `contributor` tags on the post.
    pub(crate) contributor: Vec<String>,
    /// An array of all the `invalid` tags on the post.
    pub(crate) invalid: Vec<String>,
    /// An array of all the `lore` tags on the post.
    pub(crate) lore: Vec<String>,
    /// An array of all the `meta` tags on the post.
    pub(crate) meta: Vec<String>,
}

impl Tags {
    /// Yields an iterator over references to all tag categories, avoiding allocations.
    pub(crate) fn iter_tags(&self) -> impl Iterator<Item = &str> {
        self.general
            .iter()
            .chain(&self.species)
            .chain(&self.character)
            .chain(&self.copyright)
            .chain(&self.artist)
            .chain(&self.contributor)
            .chain(&self.invalid)
            .chain(&self.lore)
            .chain(&self.meta)
            .map(|s| s.as_str())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Flags {
    /// If the post is pending approval.
    pub(crate) pending: bool,
    /// If the post is flagged for deletion.
    pub(crate) flagged: bool,
    /// If the post has it’s notes locked.
    pub(crate) note_locked: bool,
    /// If the post’s status has been locked.
    pub(crate) status_locked: Option<bool>,
    /// If the post’s rating has been locked.
    pub(crate) rating_locked: bool,
    /// If the post has been deleted.
    pub(crate) deleted: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Relationships {
    /// The ID of the post’s parent, if it has one.
    pub(crate) parent_id: Option<i64>,
    /// If the post has child posts.
    pub(crate) has_children: bool,
    pub(crate) has_active_children: bool,
    /// A list of child post IDs that are linked to the post, if it has any.
    pub(crate) children: Vec<i64>,
}

/// GET return of set entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SetEntry {
    /// The ID of the set.
    pub(crate) id: i64,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: String,
    /// The ID of the user that created the set.
    pub(crate) creator_id: i64,
    /// If the set is public and visible.
    pub(crate) is_public: bool,
    /// The name of the set.
    pub(crate) name: String,
    /// The short name of the set.
    pub(crate) shortname: String,
    /// The description of the set.
    pub(crate) description: String,
    /// The amount of posts in the set.
    pub(crate) post_count: i64,
    /// If the set will transfer its post on delete.
    pub(crate) transfer_on_delete: bool,
    /// An array group of posts in the pool.
    pub(crate) post_ids: Vec<i64>,
}

/// GET return of pool entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PoolEntry {
    /// The ID of the pool.
    pub(crate) id: i64,
    /// The name of the pool.
    pub(crate) name: String,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: String,
    /// The ID of the user that created the pool.
    pub(crate) creator_id: i64,
    /// The description of the pool.
    pub(crate) description: Option<String>,
    /// If the pool is active and still getting posts added.
    pub(crate) is_active: bool,
    /// Can be `series` or `collection`.
    pub(crate) category: String,
    /// An array group of posts in the pool.
    pub(crate) post_ids: Vec<i64>,
    /// The name of the user that created the pool.
    pub(crate) creator_name: String,
    /// The amount of posts in the pool.
    pub(crate) post_count: i64,
}

/// Subset of the user JSON response returned by `/users/me.json`.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UserEntry {
    /// The unique ID of the user.
    pub(crate) id: i64,
    /// The username of the user.
    pub(crate) name: String,
    /// The user's blacklist tags.
    pub(crate) blacklisted_tags: Option<String>,
}
