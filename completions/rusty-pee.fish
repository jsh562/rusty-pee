# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_rusty_pee_global_optspecs
	string join \n capture strict no-strict h/help V/version
end

function __fish_rusty_pee_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_rusty_pee_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_rusty_pee_using_subcommand
	set -l cmd (__fish_rusty_pee_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -l capture -d 'Buffer each child\'s stdout and emit in argv order after all children exit (Default mode only). Without this flag, children inherit the parent\'s stdout and their outputs interleave nondeterministically'
complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -l strict -d 'Enable strict moreutils-compat mode'
complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -l no-strict -d 'Explicitly disable strict mode (overrides env + argv[0])'
complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -s V -l version -d 'Print version'
complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -a "completions" -d 'Emit shell completion scripts (Default mode only)'
complete -c rusty-pee -n "__fish_rusty_pee_needs_command" -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rusty-pee -n "__fish_rusty_pee_using_subcommand completions" -s h -l help -d 'Print help'
complete -c rusty-pee -n "__fish_rusty_pee_using_subcommand help; and not __fish_seen_subcommand_from completions help" -f -a "completions" -d 'Emit shell completion scripts (Default mode only)'
complete -c rusty-pee -n "__fish_rusty_pee_using_subcommand help; and not __fish_seen_subcommand_from completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
