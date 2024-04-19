# Table Component Usage

This guide describes how to use the `Table` and associated components within SvelteKit to create a structured and styled table.

## Example

The following example demonstrates how to create a simple table with headers and multiple rows:

<Table>
  <TableHead>
    <TableRow>
      <TableCell type="th">Name</TableCell>
      <TableCell type="th">Age</TableCell>
      <TableCell type="th">Country</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    <TableRow>
      <TableCell>Alice</TableCell>
      <TableCell>25</TableCell>
      <TableCell>USA</TableCell>
    </TableRow>
    <TableRow>
      <TableCell>Bob</TableCell>
      <TableCell>30</TableCell>
      <TableCell>Canada</TableCell>
    </TableRow>
    <TableRow>
      <TableCell>Charlie</TableCell>
      <TableCell>35</TableCell>
      <TableCell>UK</TableCell>
    </TableRow>
  </TableBody>
</Table>
