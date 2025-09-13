"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Box, Button, FormControl, InputLabel, MenuItem, Select } from "@mui/material";

export default function BranchSelector({ branches }) {
  const router = useRouter();
  const [selectedBranch, setSelectedBranch] = useState(branches?.[0]?.branchid || "");

  const handleViewBranch = () => {
    if (selectedBranch) {
      router.push(`/variant/${selectedBranch}`);
    }
  };

  return (
    <Box display="flex" alignItems="start">
        <Button
            variant="contained"
            color="secondary"
            disabled={!selectedBranch}
            onClick={handleViewBranch}
        >
            View Variant
        </Button>
        <FormControl sx={{ minWidth: 150 }}>
            <InputLabel id="branch-select-label">Select Variant</InputLabel>
            <Select
            labelId="variant-select-label"
            value={selectedBranch}
            label="Select Variant"
            onChange={(e) => setSelectedBranch(e.target.value)}
            >
            {branches?.map((branch) => (
                <MenuItem key={branch.branchid} value={branch.branchid}>
                {branch.branchname}
                </MenuItem>
            ))}
            </Select>
        </FormControl>
    </Box>
  );
}