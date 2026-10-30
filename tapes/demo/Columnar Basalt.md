Some lava flows cool into tall columns that fit together like a honeycomb.

> [!tip] Try it
> Press `ctrl+o` to jump between headings with the outline.

## How the columns form

A thick flow of lava cools from its top and base. As the rock shrinks, it cracks into a network of joints. The cracks grow inward and split the flow into long prisms.

> [!note]
> Most of the columns have five, six or seven sides.

## Where to see them

| Site             | Country          |
| ---------------- | ---------------- |
| Giant's Causeway | Northern Ireland |
| Fingal's Cave    | Scotland         |
| Devils Postpile  | United States    |
| Reynisfjara      | Iceland          |

## Field checklist

- [x] Pack a rain jacket
- [x] Check the tide times
- [ ] Count the sides of ten columns

## Sketch a column

```rust
/// Most columns have five, six or seven sides.
fn is_column(sides: u8) -> bool {
    (5..=7).contains(&sides)
}

fn main() {
    let site = "Giant's Causeway";
    println!("{site}: {}", is_column(6));
}
```
