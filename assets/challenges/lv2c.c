#include <stdio.h>
int main(void){ unsigned x;
  printf("number: "); scanf("%u", &x);
  if ((x << 1) == 0x1D6) { puts("Correct! FLAG{lv2c_shl}"); return 0; } /* x = 0xEB = 235 */
  puts("Wrong"); return 1; }