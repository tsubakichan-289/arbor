# Source from ~/.bashrc after putting the arbor binary on PATH.
arbor() {
    local arbor_cwd_file arbor_directory arbor_result
    arbor_cwd_file=$(command mktemp "${TMPDIR:-/tmp}/arbor-cwd.XXXXXXXXXX") || return
    # Using an if also guarantees cleanup when the caller has set -e.
    if command arbor --cwd-file "$arbor_cwd_file" "$@"; then
        arbor_result=0
        if IFS= read -r -d '' arbor_directory < "$arbor_cwd_file"; then
            if [[ -n $arbor_directory ]]; then
                builtin cd -- "$arbor_directory" || arbor_result=$?
            fi
        fi
    else
        arbor_result=$?
    fi
    command rm -f -- "$arbor_cwd_file"
    return "$arbor_result"
}
