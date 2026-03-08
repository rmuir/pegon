
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'pegon' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'pegon'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'pegon' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Run pegon on the given files or directories')
            [CompletionResult]::new('format', 'format', [CompletionResultType]::ParameterValue, 'Run the pegon formatter on the given files or directories')
            [CompletionResult]::new('server', 'server', [CompletionResultType]::ParameterValue, 'Run the language server')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pegon;check' {
            [CompletionResult]::new('--output-format', '--output-format', [CompletionResultType]::ParameterName, 'Diagnostic output format')
            [CompletionResult]::new('--fix', '--fix', [CompletionResultType]::ParameterName, 'Apply fixes to resolve lint violations')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'pegon;format' {
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Avoid writing any formatted files back; instead, exit with a non-zero status code if any files would be modified, and zero otherwise')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pegon;server' {
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Listen on loopback TCP socket')
            [CompletionResult]::new('--stdio', '--stdio', [CompletionResultType]::ParameterName, 'Use standard I/O streams (default)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'pegon;help' {
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Run pegon on the given files or directories')
            [CompletionResult]::new('format', 'format', [CompletionResultType]::ParameterValue, 'Run the pegon formatter on the given files or directories')
            [CompletionResult]::new('server', 'server', [CompletionResultType]::ParameterValue, 'Run the language server')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'pegon;help;check' {
            break
        }
        'pegon;help;format' {
            break
        }
        'pegon;help;server' {
            break
        }
        'pegon;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
