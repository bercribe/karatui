use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::conf;

#[derive(Clone)]
pub struct Bookmark {
    id: String,
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
    pub lists: Vec<String>,
}

struct List {
    id: String,
    name: String,
}

// this is a oneof, but not sure how to represent that.
// this is fine for now.
// https://docs.karakeep.app/api/get-bookmarks-in-the-list
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BookmarkContent {
    url: Option<String>,
    source_url: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiBookmark {
    id: String,
    title: Option<String>,
    tags: Vec<Tag>,
    content: BookmarkContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BookmarksResponse {
    bookmarks: Vec<ApiBookmark>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tag {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagsResponse {
    tags: Vec<Tag>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiList {
    id: String,
    name: String,
    parent_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListsResponse {
    lists: Vec<ApiList>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateTagRequest {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTagResponse {
    id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TagUpdate {
    tag_id: String,
    tag_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TagUpdateRequest {
    tags: Vec<TagUpdate>,
}

async fn parse_response<T: DeserializeOwned>(
    response: reqwest::Response,
    context: &str,
) -> Result<T> {
    let body = response
        .text()
        .await
        .context(format!("Failed to read response body for: {}", context))?;
    serde_json::from_str(&body).context(format!("{}: {}", context, body))
}

pub async fn fetch_bookmarks(config: &conf::Config) -> Result<Vec<Bookmark>> {
    let url = &config.url;
    let key = &config.api_key;
    let list_id = &config.list_id;

    let client = reqwest::Client::builder().build()?;

    let mut api_bookmarks: Vec<ApiBookmark> = Vec::new();
    let mut next_cursor: Option<String> = None;

    loop {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/json".parse()?);
        headers.insert("Authorization", format!("Bearer {}", key).parse()?);
        let query = next_cursor.map_or("".to_string(), |s| format!("?cursor={}", s));
        let request = client
            .request(
                reqwest::Method::GET,
                format!("{}/api/v1/lists/{}/bookmarks{}", url, list_id, query),
            )
            .headers(headers);

        let response = request.send().await?;
        let body: BookmarksResponse = parse_response(response, "get bookmarks").await?;

        api_bookmarks.extend(body.bookmarks);

        next_cursor = body.next_cursor;
        if next_cursor.is_none() {
            break;
        }
    }

    let all_lists = fetch_lists(config).await?;
    let lists_map: HashMap<&String, &List> = all_lists.iter().map(|l| (&l.id, l)).collect();

    let mut bookmarks: Vec<Bookmark> = Vec::new();
    for b in api_bookmarks {
        let lists = fetch_bookmark_lists(config, &b.id, &lists_map).await?;
        let bookmark = Bookmark {
            id: b.id.clone(),
            url: b
                .content
                .url
                .as_ref()
                .or(b.content.source_url.as_ref())
                .unwrap_or(&"".to_owned())
                .clone(),
            title: b
                .title
                .as_ref()
                .or(b.content.title.as_ref())
                .unwrap_or(&format!("Title not found! ID: {}", b.id))
                .clone(),
            tags: b.tags.iter().map(|b| b.name.clone()).collect(),
            lists: lists.iter().map(|l| l.name.clone()).collect(),
        };
        bookmarks.push(bookmark);
    }

    Ok(bookmarks)
}

pub async fn fetch_available_tags(config: &conf::Config) -> Result<Vec<String>> {
    let tags = fetch_tags(config).await?;
    Ok(tags.iter().map(|t| t.name.clone()).collect())
}

pub async fn fetch_available_lists(config: &conf::Config) -> Result<Vec<String>> {
    let lists = fetch_lists(config).await?;
    Ok(lists.iter().map(|l| l.name.clone()).collect())
}

pub async fn save_bookmarks(config: &conf::Config, bookmarks: &[&Bookmark]) -> Result<()> {
    let url = &config.url;
    let key = &config.api_key;

    let tags = fetch_tags(config).await?;
    let mut tag_id_map: HashMap<String, String> = tags
        .iter()
        .map(|t| (t.name.clone(), t.id.clone()))
        .collect();
    let all_lists = fetch_lists(config).await?;
    let list_id_map: HashMap<&String, &List> = all_lists.iter().map(|l| (&l.id, l)).collect();
    let list_name_map: HashMap<&String, &List> = all_lists.iter().map(|l| (&l.name, l)).collect();

    let new_tags: Vec<&String> = bookmarks
        .iter()
        .flat_map(|b| &b.tags)
        .filter(|t| !tag_id_map.contains_key(t.to_owned()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let client = reqwest::Client::builder().build()?;
    for tag in new_tags {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("Accept", "application/json".parse()?);
        headers.insert("Authorization", format!("Bearer {}", key).parse()?);

        let request_body = CreateTagRequest { name: tag.clone() };

        let request = client
            .request(reqwest::Method::POST, format!("{}/api/v1/tags", url))
            .headers(headers)
            .json(&request_body);

        let response = request.send().await?;
        let body: CreateTagResponse =
            parse_response(response, &format!("create tag '{}'", tag)).await?;

        tag_id_map.insert(body.name, body.id);
    }

    for bookmark in bookmarks {
        // get current tag state
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/json".parse()?);
        headers.insert("Authorization", format!("Bearer {}", key).parse()?);
        let request = client
            .request(
                reqwest::Method::GET,
                format!("{}/api/v1/bookmarks/{}", url, bookmark.id),
            )
            .headers(headers);

        let response = request.send().await?;
        let body: ApiBookmark =
            parse_response(response, &format!("get bookmark '{}'", bookmark.id)).await?;

        let current_tags: Vec<&String> = body.tags.iter().map(|t| &t.name).collect();
        let tags_to_add: Vec<&String> = bookmark
            .tags
            .iter()
            .filter(|t| !current_tags.contains(t))
            .collect();
        let tags_to_delete: Vec<&&String> = current_tags
            .iter()
            .filter(|t| !bookmark.tags.contains(t))
            .collect();

        // add new tags
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("Accept", "application/json".parse()?);
        headers.insert("Authorization", format!("Bearer {}", key).parse()?);

        let request_body = TagUpdateRequest {
            tags: tags_to_add
                .iter()
                .map(|t| TagUpdate {
                    tag_id: tag_id_map[t.to_owned()].clone(),
                    tag_name: t.to_owned().to_owned(),
                })
                .collect(),
        };

        let request = client
            .request(
                reqwest::Method::POST,
                format!("{}/api/v1/bookmarks/{}/tags", url, bookmark.id),
            )
            .headers(headers)
            .json(&request_body);

        let response = request.send().await?;
        response.text().await?;

        // delete old tags
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("Accept", "application/json".parse()?);
        headers.insert("Authorization", format!("Bearer {}", key).parse()?);

        let request_body = TagUpdateRequest {
            tags: tags_to_delete
                .iter()
                .map(|t| TagUpdate {
                    tag_id: tag_id_map[t.to_owned().to_owned()].clone(),
                    tag_name: t.to_owned().to_owned().to_owned(),
                })
                .collect(),
        };

        let request = client
            .request(
                reqwest::Method::DELETE,
                format!("{}/api/v1/bookmarks/{}/tags", url, bookmark.id),
            )
            .headers(headers)
            .json(&request_body);

        let response = request.send().await?;
        response.text().await?;

        // get current list state
        let current_lists = fetch_bookmark_lists(config, &bookmark.id, &list_id_map).await?;
        let current_list_names: Vec<&String> = current_lists.iter().map(|l| &l.name).collect();
        let lists_to_add = (bookmark
            .lists
            .iter()
            .filter(|l| !current_list_names.contains(l))
            .map(|l| list_name_map.get(l).ok_or(anyhow!("No list named {}", l)))
            .collect::<Result<Vec<&&List>>>())?;
        let lists_to_delete: Vec<&List> = current_lists
            .iter()
            .filter(|l| !bookmark.lists.contains(&l.name))
            .collect();

        // add new lists
        for l in lists_to_add {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Authorization", format!("Bearer {}", key).parse()?);
            let request = client
                .request(
                    reqwest::Method::PUT,
                    format!("{}/api/v1/lists/{}/bookmarks/{}", url, l.id, bookmark.id),
                )
                .headers(headers);

            let response = request.send().await?;
            response.text().await?;
        }

        // delete old lists
        for l in lists_to_delete {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Authorization", format!("Bearer {}", key).parse()?);
            let request = client
                .request(
                    reqwest::Method::DELETE,
                    format!("{}/api/v1/lists/{}/bookmarks/{}", url, l.id, bookmark.id),
                )
                .headers(headers);

            let response = request.send().await?;
            response.text().await?;
        }
    }

    Ok(())
}

async fn fetch_tags(config: &conf::Config) -> Result<Vec<Tag>> {
    let url = &config.url;
    let key = &config.api_key;

    let client = reqwest::Client::builder().build()?;

    let mut tags: Vec<Tag> = Vec::new();
    let mut next_cursor: Option<String> = None;

    loop {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/json".parse()?);
        headers.insert("Authorization", format!("Bearer {}", key).parse()?);
        let query = next_cursor.map_or("".to_string(), |s| format!("?cursor={}", s));
        let request = client
            .request(
                reqwest::Method::GET,
                format!("{}/api/v1/tags{}", url, query),
            )
            .headers(headers);

        let response = request.send().await?;
        let body: TagsResponse = parse_response(response, "get tags").await?;

        tags.extend(body.tags);

        next_cursor = body.next_cursor;
        if next_cursor.is_none() {
            break;
        }
    }

    Ok(tags)
}

async fn fetch_lists(config: &conf::Config) -> Result<Vec<List>> {
    let url = &config.url;
    let key = &config.api_key;

    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/json".parse()?);
    headers.insert("Authorization", format!("Bearer {}", key).parse()?);
    let request = client
        .request(reqwest::Method::GET, format!("{}/api/v1/lists", url))
        .headers(headers);

    let response = request.send().await?;
    let body: ListsResponse = parse_response(response, "get lists").await?;

    let lists_map: HashMap<&String, &ApiList> = body.lists.iter().map(|l| (&l.id, l)).collect();
    let lists = body
        .lists
        .iter()
        .map(|l| {
            let mut name = l.name.clone();
            let mut parent_id = &l.parent_id;
            while let Some(p) = parent_id {
                name = format!("{}/{}", lists_map[p].name, name);
                parent_id = &lists_map[p].parent_id;
            }
            List {
                id: l.id.clone(),
                name,
            }
        })
        .collect();

    Ok(lists)
}

async fn fetch_bookmark_lists(
    config: &conf::Config,
    bookmark_id: &String,
    list_id_map: &HashMap<&String, &List>,
) -> Result<Vec<List>> {
    let url = &config.url;
    let key = &config.api_key;

    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/json".parse()?);
    headers.insert("Authorization", format!("Bearer {}", key).parse()?);
    let request = client
        .request(
            reqwest::Method::GET,
            format!("{}/api/v1/bookmarks/{}/lists", url, bookmark_id),
        )
        .headers(headers);

    let response = request.send().await?;
    let body: ListsResponse =
        parse_response(response, &format!("get bookmark '{}' lists", bookmark_id)).await?;

    let lists = body
        .lists
        .iter()
        .map(|l| List {
            id: l.id.clone(),
            name: list_id_map[&l.id].name.clone(),
        })
        .collect();

    Ok(lists)
}
