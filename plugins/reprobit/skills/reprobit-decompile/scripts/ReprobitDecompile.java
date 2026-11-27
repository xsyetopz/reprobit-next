// Prints Ghidra's C for the functions named in the script arguments, or for every
// function when none are named, as a first draft for reprobit-decompile.
// Run: analyzeHeadless <tmp project dir> draft -import <binary> -scriptPath <this dir>
//      -postScript ReprobitDecompile.java [symbol ...] -deleteProject
// @category ReproBit

import java.util.Arrays;
import java.util.List;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

public class ReprobitDecompile extends GhidraScript {
    @Override
    public void run() throws Exception {
        List<String> wanted = Arrays.asList(getScriptArgs());
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        int printed = 0;
        for (Function function : currentProgram.getFunctionManager().getFunctions(true)) {
            if (!wanted.isEmpty() && !wanted.contains(function.getName())) {
                continue;
            }
            DecompileResults results = decompiler.decompileFunction(function, 120, monitor);
            if (!results.decompileCompleted()) {
                printerr("DRAFT-FAILED " + function.getName() + ": " + results.getErrorMessage());
                continue;
            }
            println("DRAFT " + function.getName() + " " + function.getEntryPoint() + "\n"
                + results.getDecompiledFunction().getC());
            printed++;
        }
        if (printed == 0) {
            printerr("DRAFT-NONE no function matched " + wanted);
        }
        decompiler.dispose();
    }
}
