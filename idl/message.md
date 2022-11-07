# Extensions
format should be `namespace/version/key`

custom extensions is allowed

## basic gui actions
```json5
{
    // forward uuid from other message
    "limit/v1/forward" : "string",
    // reply to other message uuid
    "limit/v1/reply" : "string",
    // pin the message to the top of the chat
    "limit/v1/pin" : "bool",
    // reaction emoji
    "limit/v1/reaction" : "string",
    // delete the message uuid
    "limit/v1/delete" : "string",
    // edit the message uuid
    "limit/v1/edit" : "string",
    // contains sensitive content uuid
    "limit/v1/sensitive" : "bool",
    // viewed the message uuid
    "limit/v1/viewed" : "bool",
    // chatting tags
    "limit/v1/tags": ["string"],
}
```

## quick voice and video messages

`string` are urls
```json5
{
  "limit/v2/quick_voice": "string",
  "limit/v2/quick_video": "string",
}
```

## multimedia messages

`string` are urls 
```json5
{
  "limit/v3/thumbnails": ["string"],
  "limit/v3/images": ["string"],
  "limit/v3/videos": ["string"],
  "limit/v3/files": ["string"],
}
```
