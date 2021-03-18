## Run Lint
.PHONY: lint
lint:
	gofmt -s -w .
	goimports -w .
	go vet ./...
	golint -set_exit_status ./...

## Run Test
.PHONY: test
test: lint
	go test ./... -cover -coverprofile=cover.out
	go tool cover -html=cover.out -o cover.html

run:
	go run .

build:
	go build main.go

build_opt:
	go build -ldflags="-s -w" -trimpath -o main main.go
