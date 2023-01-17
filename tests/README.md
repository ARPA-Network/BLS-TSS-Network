To generate gRPC code:
`python -m grpc_tools.protoc -I../crates/arpa-node/proto --python_out=. --pyi_out=. --grpc_python_out=. ../crates/arpa-node/proto/some.proto`
