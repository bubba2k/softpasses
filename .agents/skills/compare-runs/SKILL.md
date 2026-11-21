# Skill goal

The goal of this skill is to compare the performance and output of two arbitrary different dev versions of the software.
 
# Expected input

The user needs to specify:
- The scene to be used for the test run
- Scene settings to be used: spp, width and height (default to 1980x1080 if not given), the number of bounces (default to 10 if not given)
- The two version to be used.

# Process

Perform a run (in release mode) for the first needed version, saving the output to a new directory.
The render result text output written to stdout, which contains performance information such as MRays per second and total render time, then needs to be remembered. We want to compare it to the other specified version.
For this purpose and another dry run (release mode) be made for the second version, again storing the output.

# Constraints

Any uncommitted changes need to temporarily be stashed, and of course retrieved after the process!!
The working directory should be in IDENTICAL states before and after running this script, save for the new output rendered to image files.

IMPORTANT: BOTH of these runs MUST be made with identical scene and scene settings. ACCOUNT FOR THIS when stashing changes or checking out older commits or branches!!

# Expected output
- A brief description of the scene used for the comparison.
- The full render info output for BOTH runs.
- A summarizing comparison between the two: Compare rendertime and kRays/s from the render info.
- The image output of both runs inside a new directory, with a short descriptive name prefixed by `comp_` inside `images/agentic/`.