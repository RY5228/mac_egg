// Benchmark "Multi4" written by ABC on Tue Aug 19 17:44:56 2025

module Multi4 (
    a0, a1, a2, a3, b0, b1, b2, b3,
    m0, m1, m2, m3, m4, m5, m6, m7  );
  input  a0, a1, a2, a3, b0, b1, b2, b3;
  output m0, m1, m2, m3, m4, m5, m6, m7;
  wire new_n18, new_n19, new_n20, new_n21, new_n23, new_n24, new_n25,
    new_n26, new_n27, new_n28, new_n29, new_n31, new_n32, new_n33, new_n34,
    new_n35, new_n36, new_n37, new_n38, new_n39, new_n40, new_n41, new_n43,
    new_n44, new_n45, new_n46, new_n47, new_n48, new_n49, new_n50, new_n51,
    new_n53, new_n54, new_n55, new_n56, new_n57, new_n58, new_n59, new_n60,
    new_n61, new_n62, new_n64, new_n65, new_n66, new_n67;
  AND2x2_ASAP7_6t_L  g00(.A(a0), .B(b0), .Y(m0));
  AND2x2_ASAP7_6t_L  g01(.A(a1), .B(b0), .Y(new_n18));
  AND3x1_ASAP7_6t_L  g02(.A(a0), .B(b1), .C(new_n18), .Y(new_n19));
  INVx1_ASAP7_6t_L   g03(.A(new_n19), .Y(new_n20));
  AO21x1_ASAP7_6t_L  g04(.A1(a0), .A2(b1), .B(new_n18), .Y(new_n21));
  AND2x2_ASAP7_6t_L  g05(.A(new_n20), .B(new_n21), .Y(m1));
  AND2x2_ASAP7_6t_L  g06(.A(a2), .B(b0), .Y(new_n23));
  AND2x2_ASAP7_6t_L  g07(.A(a1), .B(b1), .Y(new_n24));
  XNOR2x2_ASAP7_6t_L g08(.A(new_n23), .B(new_n24), .Y(new_n25));
  XOR2x2_ASAP7_6t_L  g09(.A(new_n19), .B(new_n25), .Y(new_n26));
  NAND2x1_ASAP7_6t_L g10(.A(a0), .B(b2), .Y(new_n27));
  NOR2x1_ASAP7_6t_L  g11(.A(new_n26), .B(new_n27), .Y(new_n28));
  AND2x2_ASAP7_6t_L  g12(.A(new_n26), .B(new_n27), .Y(new_n29));
  NOR2x1_ASAP7_6t_L  g13(.A(new_n28), .B(new_n29), .Y(m2));
  MAJx1_ASAP7_6t_L   g14(.A(new_n19), .B(new_n23), .C(new_n24), .Y(new_n31));
  AND2x2_ASAP7_6t_L  g15(.A(a3), .B(b0), .Y(new_n32));
  AND2x2_ASAP7_6t_L  g16(.A(a2), .B(b1), .Y(new_n33));
  XNOR2x2_ASAP7_6t_L g17(.A(new_n32), .B(new_n33), .Y(new_n34));
  XNOR2x2_ASAP7_6t_L g18(.A(new_n31), .B(new_n34), .Y(new_n35));
  AND2x2_ASAP7_6t_L  g19(.A(a1), .B(b2), .Y(new_n36));
  XNOR2x2_ASAP7_6t_L g20(.A(new_n35), .B(new_n36), .Y(new_n37));
  XOR2x2_ASAP7_6t_L  g21(.A(new_n28), .B(new_n37), .Y(new_n38));
  NAND2x1_ASAP7_6t_L g22(.A(a0), .B(b3), .Y(new_n39));
  NOR2x1_ASAP7_6t_L  g23(.A(new_n38), .B(new_n39), .Y(new_n40));
  AND2x2_ASAP7_6t_L  g24(.A(new_n38), .B(new_n39), .Y(new_n41));
  NOR2x1_ASAP7_6t_L  g25(.A(new_n40), .B(new_n41), .Y(m3));
  MAJx1_ASAP7_6t_L   g26(.A(new_n28), .B(new_n35), .C(new_n36), .Y(new_n43));
  MAJx1_ASAP7_6t_L   g27(.A(new_n31), .B(new_n32), .C(new_n33), .Y(new_n44));
  AND2x2_ASAP7_6t_L  g28(.A(a3), .B(b1), .Y(new_n45));
  XOR2x2_ASAP7_6t_L  g29(.A(new_n44), .B(new_n45), .Y(new_n46));
  AND2x2_ASAP7_6t_L  g30(.A(a2), .B(b2), .Y(new_n47));
  XNOR2x2_ASAP7_6t_L g31(.A(new_n46), .B(new_n47), .Y(new_n48));
  XNOR2x2_ASAP7_6t_L g32(.A(new_n43), .B(new_n48), .Y(new_n49));
  AND2x2_ASAP7_6t_L  g33(.A(a1), .B(b3), .Y(new_n50));
  XNOR2x2_ASAP7_6t_L g34(.A(new_n49), .B(new_n50), .Y(new_n51));
  XNOR2x2_ASAP7_6t_L g35(.A(new_n40), .B(new_n51), .Y(m4));
  MAJx1_ASAP7_6t_L   g36(.A(new_n40), .B(new_n49), .C(new_n50), .Y(new_n53));
  MAJx1_ASAP7_6t_L   g37(.A(new_n43), .B(new_n46), .C(new_n47), .Y(new_n54));
  AND2x2_ASAP7_6t_L  g38(.A(a3), .B(b2), .Y(new_n55));
  AND3x1_ASAP7_6t_L  g39(.A(new_n44), .B(new_n45), .C(new_n55), .Y(new_n56));
  AOI21x1_ASAP7_6t_L g40(.A1(new_n44), .A2(new_n45), .B(new_n55), .Y(new_n57));
  NOR2x1_ASAP7_6t_L  g41(.A(new_n56), .B(new_n57), .Y(new_n58));
  XNOR2x2_ASAP7_6t_L g42(.A(new_n54), .B(new_n58), .Y(new_n59));
  INVx1_ASAP7_6t_L   g43(.A(new_n59), .Y(new_n60));
  AND2x2_ASAP7_6t_L  g44(.A(a2), .B(b3), .Y(new_n61));
  XOR2x2_ASAP7_6t_L  g45(.A(new_n59), .B(new_n61), .Y(new_n62));
  XNOR2x2_ASAP7_6t_L g46(.A(new_n53), .B(new_n62), .Y(m5));
  MAJx1_ASAP7_6t_L   g47(.A(new_n53), .B(new_n60), .C(new_n61), .Y(new_n64));
  AO21x1_ASAP7_6t_L  g48(.A1(new_n54), .A2(new_n58), .B(new_n56), .Y(new_n65));
  AND2x2_ASAP7_6t_L  g49(.A(a3), .B(b3), .Y(new_n66));
  XNOR2x2_ASAP7_6t_L g50(.A(new_n65), .B(new_n66), .Y(new_n67));
  XNOR2x2_ASAP7_6t_L g51(.A(new_n64), .B(new_n67), .Y(m6));
  MAJx1_ASAP7_6t_L   g52(.A(new_n64), .B(new_n65), .C(new_n66), .Y(m7));
endmodule


