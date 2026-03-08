# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_pegon_global_optspecs
	string join \n h/help V/version
end

function __fish_pegon_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_pegon_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_pegon_using_subcommand
	set -l cmd (__fish_pegon_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c pegon -n "__fish_pegon_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c pegon -n "__fish_pegon_needs_command" -s V -l version -d 'Print version'
complete -c pegon -n "__fish_pegon_needs_command" -f -a "check" -d 'Run pegon on the given files or directories'
complete -c pegon -n "__fish_pegon_needs_command" -f -a "format" -d 'Run the pegon formatter on the given files or directories'
complete -c pegon -n "__fish_pegon_needs_command" -f -a "server" -d 'Run the language server'
complete -c pegon -n "__fish_pegon_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c pegon -n "__fish_pegon_using_subcommand check" -l output-format -d 'Diagnostic output format' -r -f -a "full\t''
concise\t''"
complete -c pegon -n "__fish_pegon_using_subcommand check" -l fix -d 'Apply fixes to resolve lint violations'
complete -c pegon -n "__fish_pegon_using_subcommand check" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c pegon -n "__fish_pegon_using_subcommand format" -l check -d 'Avoid writing any formatted files back; instead, exit with a non-zero status code if any files would be modified, and zero otherwise'
complete -c pegon -n "__fish_pegon_using_subcommand format" -s h -l help -d 'Print help'
complete -c pegon -n "__fish_pegon_using_subcommand server" -l socket -d 'Listen on loopback TCP socket' -r
complete -c pegon -n "__fish_pegon_using_subcommand server" -l stdio -d 'Use standard I/O streams (default)'
complete -c pegon -n "__fish_pegon_using_subcommand server" -s h -l help -d 'Print help'
complete -c pegon -n "__fish_pegon_using_subcommand help; and not __fish_seen_subcommand_from check format server help" -f -a "check" -d 'Run pegon on the given files or directories'
complete -c pegon -n "__fish_pegon_using_subcommand help; and not __fish_seen_subcommand_from check format server help" -f -a "format" -d 'Run the pegon formatter on the given files or directories'
complete -c pegon -n "__fish_pegon_using_subcommand help; and not __fish_seen_subcommand_from check format server help" -f -a "server" -d 'Run the language server'
complete -c pegon -n "__fish_pegon_using_subcommand help; and not __fish_seen_subcommand_from check format server help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
