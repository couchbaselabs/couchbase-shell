export def assert [expected, result] {
       if (not ($expected == $result)) {
       print (ansi red_bold) failed (ansi reset)
       print "EXPECTED: " ($expected | flatten)
       print "RESULT: " ($result | flatten)
   } else {
       print (ansi green_bold) passed (ansi reset)
   }
}