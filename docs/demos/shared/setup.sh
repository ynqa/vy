# Sourced by VHS from the repository root. Each recording gets fresh state.
demo_root="$PWD"
demo_binary="$demo_root/target/release/vy"
if [ ! -x "$demo_binary" ]; then
  printf '%s\n' 'Run cargo build --release --locked first.' >&2
  return 1
fi

mkdir -p "$demo_root/docs/demos/.work"
mkdir -p "$demo_root/docs/public/demos/explore" \
  "$demo_root/docs/public/demos/reference" \
  "$demo_root/docs/public/demos/explore/commands" \
  "$demo_root/docs/public/demos/explore/navigation" \
  "$demo_root/docs/public/demos/explore/search" \
  "$demo_root/docs/public/demos/explore/focus-and-jaq"
mkdir -p "$demo_root/docs/demos/.work/explore" \
  "$demo_root/docs/demos/.work/reference" \
  "$demo_root/docs/demos/.work/explore/commands" \
  "$demo_root/docs/demos/.work/explore/navigation" \
  "$demo_root/docs/demos/.work/explore/search" \
  "$demo_root/docs/demos/.work/explore/focus-and-jaq"
demo_dir=$(mktemp -d "$demo_root/docs/demos/.work/run.XXXXXX")
mkdir -p "$demo_dir/home" "$demo_dir/config" "$demo_dir/data"
cp "$demo_root"/docs/demos/fixtures/* "$demo_dir/"
cp "$demo_root/default.toml" "$demo_dir/config.toml"
cd "$demo_dir" || return 1
trap 'rm -rf "$demo_dir"' EXIT

# Scope the home and XDG directories to the viewer process, including on macOS.
# The recording shell keeps its own environment; real vy history is never read.
vy() {
  env HOME="$demo_dir/home" XDG_CONFIG_HOME="$demo_dir/config" \
    XDG_DATA_HOME="$demo_dir/data" "$demo_binary" --config "$demo_dir/config.toml" "$@"
}

export PS1='$ '
unset PROMPT_COMMAND NO_COLOR
export COLORTERM=truecolor
printf 'DEMO_READY\n'
