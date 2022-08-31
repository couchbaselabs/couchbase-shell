package main

import (
	"bufio"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
)

func main() {
	rootFlag := flag.String("root", "./tests", "path to the root tests directory")
	flag.Parse()

	var testNames []string
	err := filepath.Walk(*rootFlag+"/tests",
		func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if info.IsDir() {
				return nil
			}

			f, err := os.Open(path)
			if err != nil {
				return err
			}
			defer f.Close()

			scanner := bufio.NewScanner(f)
			for scanner.Scan() {
				line := strings.TrimSpace(scanner.Text())
				if !strings.HasPrefix(line, "pub async fn test_") {
					continue
				}

				idx := strings.Index(line, "test_")
				part := line[idx:]
				bIdx := strings.Index(part, "(")
				testNames = append(testNames, fmt.Sprintf("%s::%s", strings.TrimSuffix(info.Name(), ".rs"), part[:bIdx]))
			}

			if err := scanner.Err(); err != nil {
				return err
			}

			return nil
		})
	if err != nil {
		log.Fatal(err)
	}

	var testfns []string
	for _, name := range testNames {
		testfns = append(testfns, fmt.Sprintf("TestFn::new(\"%s\", Box::pin(%s(cluster.clone())))", name, name))
	}

	err = os.WriteFile(*rootFlag+"/test_functions.rs", []byte(fmt.Sprintf(template, strings.Join(testfns, ",\n"))), 0644)
	if err != nil {
		panic(err)
	}
}

var template = `
use crate::tests::*;
use futures::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::ClusterUnderTest;

// Sad panda noises
pub fn tests(cluster: Arc<ClusterUnderTest>) -> Vec<TestFn> {
    vec![
        %s
    ]
}

pub struct TestFn {
    pub name: String,
    pub func: Pin<Box<dyn Future<Output = bool> + Send + 'static>>,
}

impl TestFn {
    pub fn new(
        name: impl Into<String>,
        func: Pin<Box<dyn Future<Output = bool> + Send + 'static>>,
    ) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}
`
