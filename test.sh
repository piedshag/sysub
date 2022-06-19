#!/bin/bash
input="output.log"
while IFS= read -r line
do
  echo "$line"
done < "$input"
