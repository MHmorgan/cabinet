# Common zsh script functionality.

autoload -U colors && colors  # $fg and $bg

# Echo some text to stdout with bold formatting, of stdout is a terminal.
function bold {
    if [[ -t 1 ]]; then
        echo "$fg_bold[default]$*$reset_color"
    else
        echo $*
    fi
}

function info {
	echo $2 "[*] $1" >&2
}

function warn {
    if [[ -t 2 ]]; then
	    echo $2 "$fg_no_bold[yellow][!] $1$reset_color" >&2
    else
	    echo $2 "[!] $1" >&2
    fi
}

function error {
    if [[ -t 2 ]]; then
	    echo $2 "$fg_no_bold[red][!!] $1$reset_color" >&2
    else
	    echo $2 "[!!] $1" >&2
    fi
}

function bail {
	error $*
	exit 1
}

setopt EXTENDED_GLOB

# Check if a given command exists.
# Returns 0 if it exists.
#
# Usage: cmd_exists CMD
#
# CMD -> Name of command.
function cmd_exists {
    which $@ &>/dev/null
}

# Check that one or more required commands are found it path.
# Exits with error code 1 if it fails.
#
# Usage: require_commands CMD...
#
# CMD -> Name of command.
function require_commands {
    PREV=$ZSH_ERROR_COUNT
    for CMD in $@
    do
        cmd_exists $CMD || error "required command not found: $CMD"
    done
    (( $ZSH_ERROR_COUNT == $PREV )) || exit 1
}

# Set a variable value. This handles variables expected to come from
# the environment, and optionally overridden by the environment.
#
# Usage: setvar [-e|-E] NAME [VAL]
#
# NAME -> Name of variable.
# PATH -> Variable value. Not needed for environment variables.
#  -e  -> The variable should already be an environment variable.
#  -E  -> The variable may already be an environment variable, if not
#        the given value is used as a default.
#  -i  -> Allow interactively getting variable value from user. This only
#        has effect if combined with -E
function setvar {
	zparseopts -D \
        e=cmn_env_var \
        E=cmn_opt_env_var \
        i=cmn_interactive \
        -prefix:=cmn_prefix

    # Get variable value from environment (-e)
	if [[ -n "$cmn_env_var" ]]; then
		cmn_val=$(printenv -0 $1) || bail "${cmn_prefix[2]}Environment variable not found: \"$1\""
    # Or, optional get value from environment (-E)
    elif [[ -n "$cmn_opt_env_var" ]]; then
		printenv -0 $1 &>/dev/null
        if (( $? == 0 )); then
		    cmn_val=$(printenv -0 $1)
        elif [[ -n "$cmn_interactive" ]]; then
            vared -cp "Please enter value of $1: " cmn_val
        else
            cmn_val="$2"
        fi
    # Or, set value to received input
	else
		cmn_val="$2"
	fi

    export $1="$cmn_val"
}

# Set a variable to a directory, with a sanity check of the directory.
#
# Usage: setdir [-e|-E] NAME [PATH]
#
# NAME -> Name of variable.
# PATH -> Variable value. Not needed for environment variables.
#  -e  -> The variable should already be an environment variable.
#  -E  -> The variable may already be an environment variable, if not
#        the given value is used as a default.
function setdir {
    local prefix="Setting directory variable: "
    setvar --prefix $prefix $@
	test -d $cmn_val || bail "${prefix}Not a directory: \"$cmn_val\""
}

# Set a variable to a file, with a sanity check of the file.
#
# Usage: setfile [-e|-E] NAME [PATH]
#
# NAME -> Name of variable.
# PATH -> Variable value. Not needed for environment variables.
#  -e  -> The variable should already be an environment variable.
#  -E  -> The variable may already be an environment variable, if not
#        the given value is used as a default.
function setfile {
    local prefix="Setting file variable: "
    setvar --prefix $prefix $@
	test -f $cmn_val || bail "${prefix}Not a file: \"$cmn_val\""
}

# Get the value of a variable from the user and export it.
#
# Usage: input PROMPT NAME
#
# PROMPT -> Prompt text.
# NAME   -> Name of variable.
function input {
    (( $# == 2 )) || bail "'input' called with $# arguments (expects 2)"
    unset cmn_val
	vared -cp "$1" cmn_val
    export $2="$cmn_val"
}

# Make the user confirm a statement. 
# The string " (Y/n) " is appended to the prompt.
# An empty input is interpreted as "yes".
#
# Usage: confirm PROMPT
#
# PROMPT -> Prompt text.
function confirm {
    (( $# == 1 )) || bail "'confirm' called with $# arguments (expected 1)"
    unset cmn_val
    vared -cp "$1 (Y/n) " cmn_val
    [[ -z "$cmn_val" || "$cmn_val" =~ '^\s*[Yy](es?)?\s*$' ]]
}

# Print a single-line header message. The header will be formatted
# with bold text if stdout is a terminal.
#
# Usage: header LINE
#
# LINE -> Any text
function header {
    if [[ -t 1 ]]; then
        echo $fg_bold[default]
    else
        echo
    fi

    echo $*
    repeat $(echo -n $* | wc -m) echo -n "="
    echo

    if [[ -t 1 ]]; then
        echo $reset_color
    else
        echo
    fi
}



