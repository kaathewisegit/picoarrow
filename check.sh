set -e

FEATURES="half zstd lz4"

FEATURE_SETS="none $FEATURES"

for arg in "$@"; do
    if [ "$arg" = "--full" ]; then
        # Generate powerset for --full
        FEATURE_SETS="none"
        for f in $FEATURES; do
            NEW_COMBS=""
            for c in $FEATURE_SETS; do
                if [ "$c" = "none" ]; then
                    NEW_COMBS="$NEW_COMBS $f"
                else
                    NEW_COMBS="$NEW_COMBS ${c},${f}"
                fi
            done
            FEATURE_SETS="$FEATURE_SETS $NEW_COMBS"
        done
        break
    fi
done

cargo fmt --check

for features in $FEATURE_SETS; do
    if [ "$features" = "none" ]; then
        FEATURE_ARGS=""
    else
        FEATURE_ARGS="--features $features"
    fi

    cargo check --no-default-features --tests $FEATURE_ARGS
    cargo clippy --no-default-features --tests $FEATURE_ARGS
    cargo test --no-default-features $FEATURE_ARGS
done
