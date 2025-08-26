// Benchmark "add2" written by ABC on Mon Aug 25 20:24:47 2025

module add2 ( 
    a0, a1, b0, b1,
    s0, s1, s2  );
  input  a0, a1, b0, b1;
  output s0, s1, s2;
  wire new_n8, new_n9, new_n11, new_n12, new_n13, new_n14, new_n15, new_n16,
    new_n17;
  AND2x2_ASAP7_6t_L  g00(.A(a0), .B(b0), .Y(new_n8));
  NOR2x1_ASAP7_6t_L  g01(.A(a0), .B(b0), .Y(new_n9));
  NOR2x1_ASAP7_6t_L  g02(.A(new_n9), .B(new_n8), .Y(s0));
  NAND2x1_ASAP7_6t_L g03(.A(a0), .B(b0), .Y(new_n11));
  AND2x2_ASAP7_6t_L  g04(.A(a1), .B(b1), .Y(new_n12));
  NOR2x1_ASAP7_6t_L  g05(.A(a1), .B(b1), .Y(new_n13));
  NOR3x1_ASAP7_6t_L  g06(.A(new_n12), .B(new_n11), .C(new_n13), .Y(new_n14));
  NAND2x1_ASAP7_6t_L g07(.A(a1), .B(b1), .Y(new_n15));
  OR2x2_ASAP7_6t_L   g08(.A(a1), .B(b1), .Y(new_n16));
  AOI21x1_ASAP7_6t_L g09(.A1(new_n15), .A2(new_n16), .B(new_n8), .Y(new_n17));
  NOR2x1_ASAP7_6t_L  g10(.A(new_n17), .B(new_n14), .Y(s1));
  AO21x1_ASAP7_6t_L  g11(.A1(new_n16), .A2(new_n8), .B(new_n12), .Y(s2));
endmodule


