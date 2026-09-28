#include <stdio.h>
#include <string.h>
int main(void){ char buf[32]; unsigned sum = 7;
  printf("password: "); scanf("%31s", buf);
  for (int i = 0; buf[i]; i++) sum = sum * 31 + (unsigned char)buf[i];
  if (sum == 3309782995u) { puts("Correct! FLAG{lv3a_hash31}"); return 0; }
  puts("Wrong"); return 1; }