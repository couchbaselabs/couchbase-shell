#!/bin/bash

asciidoctor -a toc=left -a toclevels=4 -a source-highlighter=coderay -a stylesheet=couchbase.css -a linkcss=true index.adoc
