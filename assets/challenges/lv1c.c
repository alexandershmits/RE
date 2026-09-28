#include <stdio.h>
#include <string.h>
int main(void){ char a[16], b[16], all[33];
  printf("part1: "); scanf("%15s", a);
  printf("part2: "); scanf("%15s", b);
  strcpy(all, a); strcat(all, b);
  if (!strcmp(all, "magic" "word")) { puts("Correct! FLAG{lv1c_concat}"); return 0; }
  puts("Wrong"); return 1; }