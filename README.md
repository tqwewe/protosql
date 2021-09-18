# ProtoSQL

<h1 align="center">ProtoSQL</h1>

<div align="center">
	Validate <b>Postgres</b> databases with your <b>Protobuf</b> files.
</div>

<br>

## Usage

```bash
$ protosql --uri "postgresql:///db" --dir ./protos
```

<img src="https://raw.githubusercontent.com/Acidic9/protosql/master/terminal.png">

## Setup

Currently, you need to clone this project and build it manually.

It can be built with [Cargo](https://crates.io/).

```bash
$ git clone git@github.com:Acidic9/protosql.git && cd protosql
$ cargo install --path .
```

Finally, validate it works.

```bash
$ protosql --version
```
