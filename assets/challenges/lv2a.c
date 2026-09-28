#include <stdio.h>
int main(void){ unsigned x;
  printf("number: "); scanf("%u", &x);
  if ((x ^ 0xDEAD) == 0xBEEF) { puts("Correct! FLAG{lv2a_xor}"); return 0; }
  puts("Wrong"); return 1; }