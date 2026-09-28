#include <stdio.h>
#include <string.h>
/* keygen-mandatory: any 12-char name where sum of chars mod 97 == flag */
int main(void){ char name[32]; unsigned sum = 0;
  printf("name (12 chars): "); scanf("%31s", name);
  if (strlen(name) != 12) { puts("Wrong length"); return 1; }
  for (int i = 0; i < 12; i++) sum += name[i];
  if (sum % 97 == 42 && name[0] == 'Z') { puts("Correct! FLAG{lv4c_keygen}"); return 0; }
  puts("Wrong"); return 1; }