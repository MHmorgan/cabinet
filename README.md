Cabinet
=======

The Cabinet file server.

API
---

### Files

Method | HTTP request | Description
------ | ------------ | -----------
get | GET /files/*path* | Returns the content of a file. Supports *If-Modified-Since*, *If-Unmodified-Since*, and *If-None-Match* headers.
head | HEAD /file/*path* | Return the same headers as for GET, without the content.
put | PUT /files/*path* | Upload a file, either creating a new file or overwriting an existing. Returns either *201 Created* or *204 No Content*. Supports *If-Unmodified-Since* and *If-Match* headers.
delete | DELETE /files/*path* | Delete a file. Supports *If-Unmodified-Since* and *If-Match* headers. The file will not be deleted if it is used in a boilerplate.

### Directories

Method | HTTP request | Description
------ | ------------ | -----------
get | GET /dirs/*path* | Returns the content of the directory as a JSON array.
put | PUT /dirs/*path* | Create a directory (if it doesn't exist). Returns either *201 Created* or *204 No Content*.
delete | DELETE /dirs/*path* | Delete a directory and its content. The directory will not be deleted if it is used in a boilerplate.

### Boilerplates

Method | HTTP request | Description
------ | ------------ | -----------
get | GET /boilerplates | Return the names of all boilerplates as a JSON array.
get | GET /boilerplates/*path* | Return the boilerplate JSON object. Supports *If-Modified-Since*, *If-Unmodified-Since*, and *If-None-Match* headers.
put | PUT /boilerplates/*path* | Upload a boilerplate, either creating a new or ovewriting an existing. Returns either *201 Created* or *204 No Content*. Supports *If-Unmodified-Since* and *If-Match* headers.
delete | DELETE /boilerplates/*path* | Delete a boilerplate. Supports *If-Unmodified-Since* and *If-Match* headers.

Boilerplate JSON object is a mapping of client side file path to server side file path:

```JSON
{
  "$HOME/.vimrc" : "configs/vimrc"
}
```
