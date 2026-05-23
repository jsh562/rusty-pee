
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'rusty-pee' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'rusty-pee'
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
        'rusty-pee' {
            [CompletionResult]::new('--capture', '--capture', [CompletionResultType]::ParameterName, 'Buffer each child''s stdout and emit in argv order after all children exit (Default mode only). Without this flag, children inherit the parent''s stdout and their outputs interleave nondeterministically')
            [CompletionResult]::new('--strict', '--strict', [CompletionResultType]::ParameterName, 'Enable strict moreutils-compat mode')
            [CompletionResult]::new('--no-strict', '--no-strict', [CompletionResultType]::ParameterName, 'Explicitly disable strict mode (overrides env + argv[0])')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Emit shell completion scripts (Default mode only)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rusty-pee;completions' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rusty-pee;help' {
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Emit shell completion scripts (Default mode only)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rusty-pee;help;completions' {
            break
        }
        'rusty-pee;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
