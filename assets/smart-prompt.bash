__veyl_smart_git() {
    local line branch= oid= dirty= ahead=0 behind=0
    while IFS= read -r line; do
        case $line in
            '# branch.head '*) branch=${line#'# branch.head '} ;;
            '# branch.oid '*) oid=${line#'# branch.oid '} ;;
            '# branch.ab '*) read -r ahead behind <<< "${line#'# branch.ab '}" ;;
            '1 '*|'2 '*|'u '*|'? '*) dirty='*'; break ;;
        esac
    done < <(command git --no-optional-locks -c core.fsmonitor=false status --porcelain=v2 --branch 2>/dev/null)
    [[ -n $branch ]] || return 0
    [[ $branch == '(detached)' ]] && branch=${oid:0:8}
    printf ' [git:%s%s' "$branch" "$dirty"
    [[ $ahead =~ ^\+[1-9][0-9]*$ ]] && printf ' ↑%s' "${ahead:1}"
    [[ $behind =~ ^-[1-9][0-9]*$ ]] && printf ' ↓%s' "${behind:1}"
    printf ']'
}

__veyl_smart_capture() {
    local status=$1 now=${EPOCHREALTIME-} elapsed
    __veyl_smart_exit=
    __veyl_smart_duration=
    if (( status != 0 )); then
        printf -v __veyl_smart_exit ' [exit:%d]' "$status"
    fi
    if [[ -n ${__veyl_smart_started-} && -n $now ]]; then
        elapsed=$(( ${now/./} - ${__veyl_smart_started/./} ))
        if (( elapsed >= 1000000 )); then
            printf -v __veyl_smart_duration ' [%d.%ds]' "$((elapsed / 1000000))" "$((elapsed / 100000 % 10))"
        fi
        unset __veyl_smart_started
    fi
    return "$status"
}

__veyl_smart_install() {
    __veyl_smart_mark='>'
    __veyl_smart_mark_color=
    if (( EUID == 0 )); then
        __veyl_smart_mark='#'
        __veyl_smart_mark_color=$'\e[1;31m'
    fi
    if [[ $__veyl_prompt == *'__veyl_smart_duration'* ]] &&
        (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 3) )); then
        local timer='${| __veyl_smart_started=$EPOCHREALTIME; REPLY=; }'
        [[ ${PS0-} == *"$timer"* ]] || PS0+=$timer
    fi
    local hook='__veyl_smart_capture "$?"' existing
    for existing in "${PROMPT_COMMAND[@]}"; do
        [[ $existing == "$hook" ]] && return 0
    done
    PROMPT_COMMAND=("$hook" "${PROMPT_COMMAND[@]}")
}
