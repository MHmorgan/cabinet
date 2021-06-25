Cabinet
=======

The Cabinet file server.

API
===

GET /files/<file>
-----------------

### Request headers

* If-None-Match: One or more sha1 hash sums of file content. If any match a
  304 (Not Modified) response will be returned. 
* If-Modified-Since
* If-Unmodified-Since

### Response headers

* ETAG: the sha1 hash sum of the file content.
* Content-Type: MIME type of file. The mime type is guessed based on file extensions.
* Last-Modified: Time of last modification of the file.
