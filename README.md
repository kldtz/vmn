# VMN - spaced repetition on the command line

No-frills spaced repetition **V**ergiss**M**ein**N**icht. Hacked together in an afternoon to escape Anki.

* Add and review cards on the command line.
* Freely set the time for the next review or use the default.
* Cards are kept in simple human-readable CSV[^1] files that you can keep under version control.
* No menus or buttons, nothing but plain text.

## Installation

```bash
# Clone this repo
git clone https://github.com/kldtz/vmn.git
# Install Rust, in case you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Change into the project directory and perform a release build
cd vmn && cargo build --release

# Run the binary
 ./target/release/vmn --help
```

## Usage

To exit the program at any point, press `Ctrl` + `c`. 

### Initialize a new box of cards (CSV file)

```bash
vmn init french.csv
```

This will create a new CSV file, unless the file already exists, and allow you to add as many cards as you like.
For each card, you're prompted to enter the front and back side.
The first review is scheduled for the same day.


### Add new cards to an existing box

```bash
vmn add french.csv
``` 

You get the same prompts as with the `init` command, but `add` will complain if the file does not exist.

Sometimes it's inconvenient to add cards interactively on the command line.
For example, I had issues typing combining Unicode characters like Arabic diacritics.
You can create a simple text file with your favourite editor, where each line represents a side of a card.
Always start with the front, followed by the back side.
Obviously, the number of lines must be even in the end.
Then you feed the content of the file to `add` via stdin:

```bash
vmn add -s french.csv < your-file.txt
```

The `-s` (`--silent`) flag suppresses the prompts you get in interactive mode.




### Review cards from a box of your choice

```bash
vmn review french.csv
```

This will show you all due cards one-by-one in random order and prompt you for the number of days after which to schedule the next review.
If you don't specify a number, it doubles the time period since the last review or schedules a review on the next day if the last review happened on the same day.
Entering zero reschedules the card for the same day, so the card will re-appear in the same session, unless you exit before with `Ctrl` + `c`.
Cards are reviewed independently in both directions.


### Print statistics

```bash
vmn stats french.csv
```

Gives an overview of the latest review intervals: a long interval indicates that the card has entered your long-term memory.


### Edit cards

Open the CSV file with your favourite editor.
`|` is used as delimiter, `#` as quote, so if you avoid these characters, you don't need to bother with quoting.
Keep the CSV under version control, just in case you corrupt the file.


[^1]: For better editability, the CSV files are not compliant with RFC 4180.