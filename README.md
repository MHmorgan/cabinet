Cabinet
=======

The Cabinet file server.

CLI
---

Start server: `cabinet <IP> <PORT> <ROOT>`

API
---

### Files

Method | HTTP request | Description
------ | ------------ | -----------
get | GET /files/*path* | Returns the content of a file. Supports *If-Modified-Since* and *If-None-Match* headers. Returns *304 Not Modified* if the *If-Modified-Since* or *If-None-Match* header condition fail. If the resource isn't a file *400 Bad Request* is returned. Returns *200 OK* on success.
head | HEAD /files/*path* | Return the same headers as for GET, without the content. If the resource is a directory *400 Bad Request* is returned. Returns *200 OK* on success.
put | PUT /files/*path* | Upload a file, either creating a new file or overwriting an existing. Supports *If-Unmodified-Since* and *If-Match* headers. Returns *412 Precondition Failed* if any of the conditionals fails. If the resource already exists, but isn't a file *400 Bad Request* is returned. Returns either *201 Created* or *204 No Content*.
delete | DELETE /files/*path* | Delete a file. Supports *If-Unmodified-Since* and *If-Match* headers. Returns *412 Precondition Failed* if any of the conditionals fails. The file will not be deleted if it is used in a boilerplate, in which case *400 Bad Request* is returned. Also, if the resource exists but isn't a file *400 Bad Request* is returned. Returns *204 No Content* on success.

### Directories

Method | HTTP request | Description
------ | ------------ | -----------
get | GET /dirs/*path* | Returns the content of the directory as a JSON array. If the resource isn't a directory *400 Bad Request* is returned. Returns *200 OK* on success.
put | PUT /dirs/*path* | Create a directory (if it doesn't exist). If the resource exists but isn't a directory *400 Bad Request* is returned. Returns either *201 Created* or *204 No Content* on success.
delete | DELETE /dirs/*path* | Delete a directory and its content. If the resource isn't a directory *400 Bad Request* is returned. The directory will not be deleted if it is used in a boilerplate, in which case *400 Bad Request* is returned. Returns *204 No Content* on success.

### Boilerplates

Method | HTTP request | Description
------ | ------------ | -----------
get | GET /boilerplates | Return the names of all boilerplates as a JSON array. Returns *200 OK* on success.
get | GET /boilerplates/*path* | Return the boilerplate JSON object. Supports *If-Modified-Since* and *If-None-Match* headers. Returns *304 Not Modified* if the *If-Modified-Since* or *If-None-Match* header condition fail. Returns *200 OK* on success.
put | PUT /boilerplates/*path* | Upload a boilerplate, either creating a new or ovewriting an existing. Returns either *201 Created* or *204 No Content*. Supports *If-Unmodified-Since* and *If-Match* headers, returning *412 Precondition Failed* if any of these fails. All files referenced in the boilerplate must be present on the server, or else *400 Bad Request* is returned.
delete | DELETE /boilerplates/*path* | Delete a boilerplate. Supports *If-Unmodified-Since* and *If-Match* headers, returning *412 Precondition Failed* if any of these fails. Returns *204 No Content* on success.

Boilerplate JSON object is a mapping of client side file path to server side file path:

```JSON
{
  "$HOME/.vimrc" : "configs/vimrc"
}
```

Clients
-------

* [commode](https://github.com/MHmorgan/commode)

Future improvements
-------------------

If time allows...

* Add file mode to boilerplate objects (will break clients)
