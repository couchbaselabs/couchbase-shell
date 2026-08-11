let bot1_role = open bot1_role.txt;
let bot2_role = open bot2_role.txt;

def bot [] {
 mut response_bot1 = "";
 mut response_bot2 = "";
 for $x in 1..6 {
    print ($"****************** ITERATION ($x) ***************")
    let rep = ask --prompt $bot1_role $response_bot1
    $response_bot1 =  $rep
    $response_bot2 =  $rep
    print ($"WRITER:\n ($response_bot1)")

    let rep2 = ask --prompt $bot2_role $response_bot2
    $response_bot1 =  $rep2
    $response_bot2 =  $rep
    print ($"EDITOR:\n ($response_bot2)")
 }
}

bot
