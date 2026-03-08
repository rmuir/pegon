
use builtin;
use str;

set edit:completion:arg-completer[pegon] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'pegon'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'pegon'= {
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
            cand check 'Run pegon on the given files or directories'
            cand format 'Run the pegon formatter on the given files or directories'
            cand server 'Run the language server'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pegon;check'= {
            cand --output-format 'Diagnostic output format'
            cand --fix 'Apply fixes to resolve lint violations'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'pegon;format'= {
            cand --check 'Avoid writing any formatted files back; instead, exit with a non-zero status code if any files would be modified, and zero otherwise'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pegon;server'= {
            cand --socket 'Listen on loopback TCP socket'
            cand --stdio 'Use standard I/O streams (default)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'pegon;help'= {
            cand check 'Run pegon on the given files or directories'
            cand format 'Run the pegon formatter on the given files or directories'
            cand server 'Run the language server'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'pegon;help;check'= {
        }
        &'pegon;help;format'= {
        }
        &'pegon;help;server'= {
        }
        &'pegon;help;help'= {
        }
    ]
    $completions[$command]
}
