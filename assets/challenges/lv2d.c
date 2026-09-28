#include <stdio.h>
int main(void){ unsigned x;
  printf("number: "); scanf("%u", &x);
  if ((x >> 4) == 0xC0DE) { puts("Correct! FLAG{lv2d_shr}"); return 0; }
  puts("Wrong"); return 1; }