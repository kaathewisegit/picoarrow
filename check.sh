set -e

FEATURE_SETS="none zstd lz4 zstd,lz4"

for features in $FEATURE_SETS; do
    if [ "$features" = "none" ]; then
        cargo check --no-default-features --tests
        cargo clippy --no-default-features --tests
        cargo test --no-default-features
    else
        cargo check --no-default-features --tests --features "$features"
        cargo clippy --no-default-features --tests --features "$features"
        cargo test --no-default-features --features "$features"
    fi
done
