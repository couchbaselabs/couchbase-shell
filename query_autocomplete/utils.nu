export def assert [expected, result] {
       if (not ($expected == $result)) {
       print (ansi red_bold) failed (ansi reset)
       print "EXPECTED: " $expected
       print "RESULT: " $result
   } else {
       print (ansi green_bold) passed (ansi reset)
   }
}