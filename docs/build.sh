#!/bin/bash

asciidoctor -a toc=left -a toclevels=4 -a source-highlighter=coderay index.adoc

