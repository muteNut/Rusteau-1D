# Command line program
keywords: command line; CLI; batch; rusteau-cli; scripting; export help

The command line program `rusteau-cli` (`rusteau-cli.exe` on Windows) runs projects without the interface, e.g. in scripts or for many scenarios.

```
rusteau-cli <project-dir | name.h1d | project.h1dr> [options]
```

| Option | Meaning |
|---|---|
| `-o`, `--out DIR` | output directory (default: `<project-dir>/rust_out`, or `<file>.results`) |
| `-c`, `--compare [DIR]` | compare all columns of the HYDRUS output files in DIR (default: the project directory); exit code 4 on differences |
| `-q`, `--quiet` | no progress display |
| `-s`, `--summary` | summary of the run |
| `--json FILE` | complete results as JSON (readable by the interface) |
| `--direct` | run an inverse project as a direct problem with the initial estimates |
| `--hydrus-stop` | parameter estimation: iterate exactly as HYDRUS 4.17 does |
| `--starts N` | parameter estimation: number of additional starting points (default 8) |
| `--save-h1d FILE` | write the project as a HYDRUS-1D 4.x project (FILE.h1d + folder FILE) and exit |

The program accepts a HYDRUS-1D project folder (the folder with `SELECTOR.IN`), the HYDRUS-1D 4.x project file `name.h1d`, or a Rusteau project `.h1dr`. It writes the [output files](output_files) `T_LEVEL.csv`, `A_LEVEL.csv`, `NOD_INF.csv`, `OBS_NODE.csv`, `BALANCE.csv` and, when produced, `T_LEVEL1.csv`, `METEO.csv`, `Meteo.out` and `IRRIG.csv`. A project with an inverse problem is optimized: the iterations are printed, and `Fit.out` and the results of the final run with the fitted parameters are written.

## Writing the HTML help

The interface program writes the help as HTML pages and exits:

```
rusteau --export-help DIR
```

Open `DIR/index.html` in a web browser. *Help → Export the HTML help…* in the interface does the same.
