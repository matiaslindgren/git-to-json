# git-to-json

Convert git commit data from `git log` lines to json/csv/postgres.

## Usage

```bash
git2json /path/to/repository json > commits.json
git2json /path/to/repository csv > commits.csv
git2json /path/to/repository postgres > create_commits.sql
```

## Example

```bash
cargo install --path ./git2json
git clone --depth 10 https://github.com/python/cpython.git
git2json ./cpython json
```
Output:
```json
{"hash":"26cca8067bf5306e372c0e90036d832c5021fd90","author_date":"2022-03-26T16:29:02+00:00","author_email":"Pablogsal@gmail.com","commit_date":"2022-03-26T09:29:02-07:00","files_changed":2,"insertions":9,"deletions":2}
{"hash":"ee912ad6f66bb8cf5a8a2b4a7ecd2752bf070864","author_date":"2022-03-25T20:09:40-04:00","author_email":"aphedges@users.noreply.github.com","commit_date":"2022-03-25T20:09:40-04:00","files_changed":1,"insertions":1,"deletions":1}
{"hash":"bad6ffaa64eecd33f4320ca31b1201b25cd8fc91","author_date":"2022-03-26T00:26:23+02:00","author_email":"andrew.svetlov@gmail.com","commit_date":"2022-03-26T00:26:23+02:00","files_changed":3,"insertions":8,"deletions":8}
{"hash":"d03acd7270d66ddb8e987f9743405147ecc15087","author_date":"2022-03-25T23:01:21+01:00","author_email":"yduprat@gmail.com","commit_date":"2022-03-26T00:01:21+02:00","files_changed":6,"insertions":856,"deletions":5}
{"hash":"20e6e5636a06fe5e1472062918d0a302d82a71c3","author_date":"2022-03-25T19:59:29+02:00","author_email":"andrew.svetlov@gmail.com","commit_date":"2022-03-25T19:59:29+02:00","files_changed":1,"insertions":1,"deletions":1}
{"hash":"c07ca1aab6e1928e9eefe9dfec7e7e5ae982b420","author_date":"2022-03-25T10:32:05-07:00","author_email":"contactme@kurtmckee.org","commit_date":"2022-03-25T10:32:05-07:00","files_changed":1,"insertions":4,"deletions":4}
{"hash":"cca43b7d64f47ea921d0f7a347ae1a839c5463c3","author_date":"2022-03-25T12:13:19-04:00","author_email":"36520290+sweeneyde@users.noreply.github.com","commit_date":"2022-03-25T16:13:19+00:00","files_changed":4,"insertions":11,"deletions":7}
{"hash":"d7163bb35d1ed46bde9affcd4eb267dfd0b703dd","author_date":"2022-03-25T12:57:50+00:00","author_email":"mark@hotpy.org","commit_date":"2022-03-25T12:57:50+00:00","files_changed":7,"insertions":46,"deletions":10}
{"hash":"b68431fadb3150134ac6ccbf501cdfeaf4c75678","author_date":"2022-03-25T00:45:50+01:00","author_email":"ezio.melotti@gmail.com","commit_date":"2022-03-25T00:45:50+01:00","files_changed":1,"insertions":5,"deletions":0}
{"hash":"8a0a9e5b1928fab7d9819c8d6498ef5c0b9383af","author_date":"2022-03-24T23:09:42+02:00","author_email":"christian@python.org","commit_date":"2022-03-24T14:09:42-07:00","files_changed":4673,"insertions":2355291,"deletions":0}
```
