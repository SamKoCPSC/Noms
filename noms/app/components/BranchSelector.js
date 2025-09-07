"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Box, Button, FormControl, InputLabel, MenuItem, Select } from "@mui/material";

export default function BranchSelector({ branches }) {
  const router = useRouter();
  const [selectedBranch, setSelectedBranch] = useState(branches?.[0]?.branchid || "");

  const handleViewBranch = () => {
    if (selectedBranch) {
      router.push(`/branch/${selectedBranch}`);
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
            View Branch
        </Button>
        <FormControl alignItems="center" justifyContent="center" sx={{ minWidth: 150 }}>
            <InputLabel id="branch-select-label">Select Branch</InputLabel>
            <Select
            labelId="branch-select-label"
            value={selectedBranch}
            label="Select Branch"
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