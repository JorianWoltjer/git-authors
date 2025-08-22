# git-authors

**Enumerate authors in Git logs of large sets of repositories for OSINT, to find names and emails.**

![Demonstration of getting authors from JorianWoltjer's repositories](docs/demo.gif)

Git stores commits with an "author", consisting of a name and email. These can't easily be viewed on UIs like GitHub, but need to be cloned to view the raw data. This simple tool implements a fast multi-threaded solution that outputs all unique authors from the given repositories.

> [!NOTE]   
> Not all results are guaranteed to be aliases of *one person*, as multiple people can work on the same repository via Collaborators and Pull Requests. Evaluate the results manually for matches.

## Installation

```bash
cargo install gitauthors
```

Or **download** and **extract** a pre-compiled binary from the [Releases](https://github.com/JorianWoltjer/git-authors/releases) page.

## Usage

You can pass any of the following types of URLs into this tool:

* Any Git URL (eg. `https://github.com/JorianWoltjer/git-authors.git` or `git@github.com:JorianWoltjer/git-authors.git`)
* GitHub Users (eg. `https://github.com/JorianWoltjer`)
* GitHub Organizations (eg. `https://github.com/twitter`)

The simplest usage is passing such a URL as an argument (multiple supported):

```bash
gitauthors https://github.com/JorianWoltjer/git-authors
```

If no arguments are provided, it listens on **stdin** for newline-separated URLs. This lets you pipe output form other commands (`|`) or files (`<`) into it.

```bash
gitauthors < urls.txt
```

The `-t` option sets the number of threads to clone with. You may have enough bandwidth to increase it for faster downloading, but the default of 10 should be pretty quick already.

```shell
$ gitauthors --help
Usage: gitauthors [OPTIONS] [URLS]...

Arguments:
  [URLS]...  URLs of repositories, users or orgnizations

Options:
  -t, --threads <THREADS>  Number of simultaneous threads to clone with [default: 10]
  -h, --help               Print help
```
