GET https://api.nasa.gov/planetary/apod?api_key=DEMO_KEY
HTTP 200
[Captures]
url: jsonpath "$.url"
hdurl: jsonpath "$.hdurl"

GET {{url}}
[Options]
output: url.png
HTTP 200


GET {{hdurl}}
[Options]
output: hdurl.png
HTTP 200

